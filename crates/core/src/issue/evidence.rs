//! Evidence extraction for findings.
//!
//! Evidence provides the surrounding code context for a finding, making the
//! output self-contained for AI consumption and human review.
//!
//! ## Fields
//!
//! - `matched_code` — the flagged code snippet (from scanner or line-window)
//! - `context_before` — lines before the finding
//! - `context_after` — lines after the finding
//! - `enclosing_function` — function/method name extracted via tree-sitter (Phase 3, Plan 02)
//! - `enclosing_class` — class/struct name extracted via tree-sitter (Phase 3, Plan 02)
//! - `imports` — file-level import statements extracted via tree-sitter (Phase 3, Plan 02)

use serde::{Deserialize, Serialize};

/// Pre-embedded code evidence for a finding.
///
/// Designed to be serialized into the AI-ready JSON output so that an LLM
/// can reason about the finding without access to the source repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceBlock {
    /// The flagged code snippet as reported by the scanner.
    /// May be an empty string when the scanner provided no snippet.
    pub matched_code: String,

    /// Lines immediately before the finding (up to ±5 lines by default).
    pub context_before: Vec<String>,

    /// Lines immediately after the finding.
    pub context_after: Vec<String>,

    /// Name of the function/method that contains the finding.
    /// `None` when at module level or when tree-sitter parsing failed.
    pub enclosing_function: Option<String>,

    /// Name of the class/struct/interface that contains the finding.
    /// `None` when at module level or when tree-sitter parsing failed.
    pub enclosing_class: Option<String>,

    /// File-level import/use statements extracted from the source file.
    /// Empty when tree-sitter parsing failed or file has no imports.
    pub imports: Vec<String>,
}

impl EvidenceBlock {
    /// Build an `EvidenceBlock` from a scanner-provided code snippet and
    /// a surrounding line window extracted from the source file.
    ///
    /// This is the line-window-only constructor used until tree-sitter
    /// extraction is implemented in Plan 03-02.
    pub fn from_line_window(
        matched_code: String,
        context_before: Vec<String>,
        context_after: Vec<String>,
    ) -> Self {
        Self {
            matched_code,
            context_before,
            context_after,
            enclosing_function: None,
            enclosing_class: None,
            imports: vec![],
        }
    }
}

/// Number of context lines to include above and below the finding line.
pub const CONTEXT_LINES: usize = 5;

/// Extract an [`EvidenceBlock`] for a finding using line-window strategy.
///
/// Tree-sitter extraction (enclosing_function, enclosing_class, imports) is
/// implemented in Plan 03-02. This function provides the line-window portion.
///
/// # Arguments
/// * `scanner_code` – value of `RawFinding::matched_code`; used as `matched_code`.
/// * `file_path` – relative source file path; joined with `project_root`.
/// * `line` – 1-based line number (0 means unknown).
/// * `project_root` – working directory of the scan.
pub fn extract_evidence_block(
    scanner_code: Option<&str>,
    file_path: &str,
    line: u32,
    project_root: &std::path::Path,
) -> EvidenceBlock {
    let matched_code = scanner_code.unwrap_or("").to_string();

    // Line-window: read the source file and extract surrounding context.
    if line > 0 && file_path != "<project>" {
        let full_path = project_root.join(file_path);
        if let Ok(source) = std::fs::read_to_string(&full_path) {
            let lines: Vec<&str> = source.lines().collect();
            // Convert from 1-based SARIF line to 0-based Vec index.
            let center = (line as usize).saturating_sub(1);
            let start = center.saturating_sub(CONTEXT_LINES);
            let end = (center + CONTEXT_LINES + 1).min(lines.len());

            let context_before: Vec<String> = if start < center {
                lines[start..center].iter().map(|s| s.to_string()).collect()
            } else {
                vec![]
            };

            // Matched code from source if scanner didn't provide it.
            let code = if matched_code.trim().is_empty() && center < lines.len() {
                lines[center].to_string()
            } else {
                matched_code.clone()
            };

            let context_after: Vec<String> = if center + 1 < end {
                lines[center + 1..end]
                    .iter()
                    .map(|s| s.to_string())
                    .collect()
            } else {
                vec![]
            };

            return EvidenceBlock::from_line_window(code, context_before, context_after);
        }
    }

    // Fallback: just the matched code from the scanner, no context.
    EvidenceBlock::from_line_window(matched_code, vec![], vec![])
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
    fn scanner_code_used_as_matched_code() {
        let tmp = write_temp("fn foo() {}\n");
        let root = tmp.path().parent().unwrap();
        let block = extract_evidence_block(
            Some("let x = eval(input)"),
            tmp.path().file_name().unwrap().to_str().unwrap(),
            1,
            root,
        );
        assert_eq!(block.matched_code, "let x = eval(input)");
    }

    #[test]
    fn context_lines_extracted() {
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
        let block = extract_evidence_block(None, &filename, 3, root);
        // context_before should have lines 1 and 2
        assert!(
            !block.context_before.is_empty(),
            "context_before should be populated"
        );
        // context_after should have lines 4 and 5
        assert!(
            !block.context_after.is_empty(),
            "context_after should be populated"
        );
    }

    #[test]
    fn empty_block_for_unknown_file() {
        let block = extract_evidence_block(None, "does_not_exist.py", 0, std::path::Path::new("."));
        assert_eq!(block.matched_code, "");
        assert!(block.context_before.is_empty());
        assert!(block.context_after.is_empty());
        assert!(block.enclosing_function.is_none());
        assert!(block.enclosing_class.is_none());
        assert!(block.imports.is_empty());
    }

    #[test]
    fn from_line_window_constructor() {
        let block = EvidenceBlock::from_line_window(
            "eval(x)".to_string(),
            vec!["# before".to_string()],
            vec!["# after".to_string()],
        );
        assert_eq!(block.matched_code, "eval(x)");
        assert!(block.enclosing_function.is_none());
        assert!(block.enclosing_class.is_none());
    }
}
