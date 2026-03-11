//! `IssueBuilder` — joins `RawFinding` + evidence + fix + priority into an `Issue`.
//!
//! This is the single integration point for all sub-modules; callers should
//! only need to interact with `IssueBuilder::build(...)`.

use std::path::Path;

use super::{evidence::extract_evidence, fix::FixMetadata, priority::compute_priority, Issue};
use crate::scanner::parser::RawFinding;

/// Builds a fully-enriched [`Issue`] from a [`RawFinding`].
///
/// ```rust
/// # use rice_guard_core::issue::builder::IssueBuilder;
/// # use rice_guard_core::scanner::parser::RawFinding;
/// # use std::path::Path;
/// // let finding: RawFinding = ...;
/// // let issue = IssueBuilder::build(&finding, Path::new("."));
/// ```
pub struct IssueBuilder;

impl IssueBuilder {
    /// Construct an [`Issue`] from `finding`, reading source files from `project_root`.
    pub fn build(finding: &RawFinding, project_root: &Path) -> Issue {
        let evidence = extract_evidence(
            finding.matched_code.as_deref(),
            &finding.file_path,
            finding.line,
            project_root,
        );

        let fix = FixMetadata::from_finding(
            &finding.scanner,
            &finding.rule_id,
            finding.suggested_replacement.as_deref(),
        );

        let has_autofix = finding.suggested_replacement.is_some();
        let priority = compute_priority(&finding.severity, &finding.scanner, has_autofix);

        let id = make_id(finding);

        Issue {
            id,
            scanner: finding.scanner.clone(),
            rule_id: finding.rule_id.clone(),
            severity: finding.severity.clone(),
            file_path: finding.file_path.clone(),
            line: finding.line,
            message: finding.message.clone(),
            evidence,
            fix,
            priority,
        }
    }
}

/// Construct a deterministic, human-readable ID for deduplication.
///
/// Format: `<scanner>/<rule_id>@<file>:<line>`
///
/// This is intentionally stable across runs so that issue trackers can
/// correlate the same finding across CI scans.
fn make_id(f: &RawFinding) -> String {
    format!("{}/{}@{}:{}", f.scanner, f.rule_id, f.file_path, f.line)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_finding(scanner: &str, severity: &str, line: u32) -> RawFinding {
        RawFinding {
            scanner: scanner.to_string(),
            rule_id: "test-rule".to_string(),
            severity: severity.to_string(),
            file_path: "src/lib.rs".to_string(),
            line,
            message: "test message".to_string(),
            matched_code: None,
            suggested_replacement: None,
        }
    }

    #[test]
    fn id_is_stable() {
        let f = make_finding("semgrep", "warning", 42);
        let id = make_id(&f);
        assert_eq!(id, "semgrep/test-rule@src/lib.rs:42");
    }

    #[test]
    fn build_produces_unknown_evidence_for_missing_file() {
        let f = make_finding("clippy", "error", 10);
        let issue = IssueBuilder::build(&f, Path::new("/tmp/nonexistent_project_xyz"));
        assert!(matches!(issue.evidence, Evidence::Unknown));
    }

    #[test]
    fn build_priority_is_high_for_clippy_error() {
        let f = make_finding("clippy", "error", 1);
        let issue = IssueBuilder::build(&f, Path::new("."));
        // error + non-secrets scanner → at least Medium
        use super::super::priority::PriorityLevel;
        assert!(issue.priority.level >= PriorityLevel::Medium);
    }
}
