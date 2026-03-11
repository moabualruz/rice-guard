//! Fix metadata for code findings.
//!
//! Captures what kind of automated fix is applicable (if any) and which tool
//! can apply it.  This is surfaced in the AI-ready JSON output so that an
//! orchestration layer can route issues to the appropriate fixer without
//! re-running the scanner.

use serde::{Deserialize, Serialize};

/// Classification of the available fix for a finding.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FixCategory {
    /// No automated fix is known; manual review required.
    Manual,
    /// The scanner emitted a one-shot replacement snippet (`suggested_replacement`).
    ScannerAutofix,
    /// A formatter/linter can fix this with a single command (e.g. `rustfmt`).
    FormatterApplicable,
    /// A `cargo fix`, `npm --fix`, or similar package-manager auto-apply works.
    PackageManagerFix,
}

/// The tool or command that can apply the fix.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AutoFixTool {
    Semgrep,
    RustFmt,
    Clippy,
    Eslint,
    Prettier,
    CargoFix,
    NpmAuditFix,
    None,
}

/// Fix information attached to a finding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixMetadata {
    /// High-level categorisation of the available fix.
    pub category: FixCategory,
    /// Tool that can apply the fix automatically.
    pub tool: AutoFixTool,
    /// Ready-to-apply replacement snippet, when available.
    pub snippet: Option<String>,
    /// Shell command the user can run to apply the fix, when applicable.
    pub command: Option<String>,
}

impl FixMetadata {
    /// Build fix metadata from scanner fields and rule identifiers.
    ///
    /// Resolution order:
    /// 1. If `suggested_replacement` is present → `ScannerAutofix` via Semgrep.
    /// 2. If `rule_id` starts with `clippy::` → `FormatterApplicable` via Clippy.
    /// 3. Else → `Manual`.
    pub fn from_finding(scanner: &str, rule_id: &str, suggested_replacement: Option<&str>) -> Self {
        if let Some(snippet) = suggested_replacement {
            return Self {
                category: FixCategory::ScannerAutofix,
                tool: AutoFixTool::Semgrep,
                snippet: Some(snippet.to_string()),
                command: Some(format!("semgrep --autofix --config={} <file>", rule_id)),
            };
        }

        if scanner == "clippy" || rule_id.starts_with("clippy::") {
            return Self {
                category: FixCategory::FormatterApplicable,
                tool: AutoFixTool::Clippy,
                snippet: None,
                command: Some("cargo clippy --fix --allow-dirty".to_string()),
            };
        }

        Self {
            category: FixCategory::Manual,
            tool: AutoFixTool::None,
            snippet: None,
            command: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scanner_autofix_when_replacement_present() {
        let meta = FixMetadata::from_finding(
            "semgrep",
            "python.lang.security.eval-injection",
            Some("ast.literal_eval(user_input)"),
        );
        assert_eq!(meta.category, FixCategory::ScannerAutofix);
        assert_eq!(meta.tool, AutoFixTool::Semgrep);
        assert!(meta.snippet.is_some());
        assert!(meta.command.unwrap().contains("semgrep"));
    }

    #[test]
    fn clippy_fix_detected() {
        let meta = FixMetadata::from_finding("clippy", "clippy::needless_return", None);
        assert_eq!(meta.category, FixCategory::FormatterApplicable);
        assert_eq!(meta.tool, AutoFixTool::Clippy);
    }

    #[test]
    fn manual_when_no_known_fix() {
        let meta = FixMetadata::from_finding("trivy", "CVE-2023-12345", None);
        assert_eq!(meta.category, FixCategory::Manual);
        assert_eq!(meta.tool, AutoFixTool::None);
    }
}
