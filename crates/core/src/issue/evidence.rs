//! Evidence extraction for findings.
//!
//! Evidence provides the surrounding code context for a finding, making the
//! output self-contained for AI consumption and human review.
//!
//! ## Extraction Strategy
//!
//! 1. **Scanner-provided** (`RawFinding::matched_code`): use as-is; highest
//!    fidelity since the scanner already knows the exact match boundary.
//! 2. **Line-window fallback**: read the source file and return N lines
//!    centred on `RawFinding::line`. Used when the scanner did not supply
//!    matched code or when `line > 0`.
//! 3. **Unknown** (`Evidence::Unknown`): emitted when the file cannot be
//!    read or the line is 0 and no scanner code exists.

use std::path::Path;

use serde::{Deserialize, Serialize};

/// Code evidence attached to a finding.
///
/// Designed to be serialized into the AI-ready JSON output so that an LLM
/// can reason about the finding without access to the source repository.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Evidence {
    /// Code provided directly by the scanner (highest fidelity).
    ScannerProvided {
        /// Raw snippet exactly as the scanner reported it.
        code: String,
    },
    /// A window of source lines centred on the finding's line number.
    LineWindow {
        /// The 1-based line number at the centre of the window.
        center_line: u32,
        /// Lines before the finding (inclusive of `center_line`).
        before: u8,
        /// Lines after the finding.
        after: u8,
        /// The extracted source text (may span multiple lines).
        code: String,
    },
    /// Evidence could not be extracted (file unreadable, line = 0, etc.).
    Unknown,
}

impl Evidence {
    /// Return the code snippet regardless of variant, or `None` for `Unknown`.
    pub fn code(&self) -> Option<&str> {
        match self {
            Self::ScannerProvided { code } | Self::LineWindow { code, .. } => Some(code.as_str()),
            Self::Unknown => None,
        }
    }
}

/// Number of context lines to include above and below the finding line.
const CONTEXT_LINES: usize = 3;

/// Extract [`Evidence`] for a finding.
///
/// # Arguments
/// * `scanner_code` – value of `RawFinding::matched_code`; used first.
/// * `file_path` – relative source file path; joined with `project_root`.
/// * `line` – 1-based line number (0 means unknown).
/// * `project_root` – working directory of the scan.
pub fn extract_evidence(
    scanner_code: Option<&str>,
    file_path: &str,
    line: u32,
    project_root: &Path,
) -> Evidence {
    // Strategy 1: scanner already provided the match.
    if let Some(code) = scanner_code {
        if !code.trim().is_empty() {
            return Evidence::ScannerProvided {
                code: code.to_string(),
            };
        }
    }

    // Strategy 2: line-window via the source file.
    if line > 0 && file_path != "<project>" {
        let full_path = project_root.join(file_path);
        if let Ok(source) = std::fs::read_to_string(&full_path) {
            let lines: Vec<&str> = source.lines().collect();
            let center = (line as usize).saturating_sub(1); // convert to 0-based
            let start = center.saturating_sub(CONTEXT_LINES);
            let end = (center + CONTEXT_LINES + 1).min(lines.len());
            if start < end {
                let code = lines[start..end].join("\n");
                return Evidence::LineWindow {
                    center_line: line,
                    before: (center - start) as u8,
                    after: (end - center - 1) as u8,
                    code,
                };
            }
        }
    }

    Evidence::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn write_temp(content: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
    }

    #[test]
    fn scanner_provided_takes_priority() {
        let tmp = write_temp("fn foo() {}\n");
        let root = tmp.path().parent().unwrap();
        let ev = extract_evidence(
            Some("let x = eval(input)"),
            tmp.path().file_name().unwrap().to_str().unwrap(),
            1,
            root,
        );
        assert!(matches!(ev, Evidence::ScannerProvided { .. }));
        assert_eq!(ev.code().unwrap(), "let x = eval(input)");
    }

    #[test]
    fn line_window_fallback() {
        let src = "line1\nline2\nline3\nline4\nline5\n";
        let tmp = write_temp(src);
        let root = tmp.path().parent().unwrap();
        let filename = tmp
            .path()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        let ev = extract_evidence(None, &filename, 3, root);
        assert!(matches!(ev, Evidence::LineWindow { center_line: 3, .. }));
        let code = ev.code().unwrap();
        assert!(code.contains("line3"));
    }

    #[test]
    fn unknown_when_no_line_and_no_scanner_code() {
        let ev = extract_evidence(None, "does_not_exist.py", 0, Path::new("."));
        assert_eq!(ev, Evidence::Unknown);
    }

    #[test]
    fn blank_scanner_code_falls_through_to_line_window() {
        let src = "a\nb\nc\n";
        let tmp = write_temp(src);
        let root = tmp.path().parent().unwrap();
        let filename = tmp
            .path()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        // Empty/whitespace scanner code should not be used.
        let ev = extract_evidence(Some("   "), &filename, 2, root);
        assert!(matches!(ev, Evidence::LineWindow { .. }));
    }
}
