//! Output serialization — produces the AI-ready JSON report files.
//!
//! # Output Files
//!
//! ```text
//! reports/<project>/<timestamp>/
//! ├── issues.json              # All issues, priority-sorted descending
//! ├── issues-fixable.json      # Issues where fix.auto_fixable = true
//! ├── issues-remaining.json    # Issues where fix.auto_fixable = false
//! ├── summary.json             # Scan metadata and counts
//! └── summary.txt              # Human-readable ASCII table
//! ```

pub mod files;
pub mod summary;

pub use summary::ScanSummary;

use std::path::{Path, PathBuf};

use crate::issue::Issue;

/// Writes all output files from a completed scan.
///
/// # Example
///
/// ```rust,no_run
/// use rguard_core::output::{OutputWriter, ScanSummary};
///
/// let writer = OutputWriter::new(std::path::Path::new("/tmp/scan-out"));
/// // writer.write_all(&issues, &summary).unwrap();
/// ```
pub struct OutputWriter {
    dir: PathBuf,
}

impl OutputWriter {
    /// Create an `OutputWriter` that writes to `dir`.
    ///
    /// The directory must already exist (created by `OutputDir::new`).
    pub fn new(dir: &Path) -> Self {
        Self {
            dir: dir.to_path_buf(),
        }
    }

    /// Write all output files for a completed scan.
    ///
    /// Produces:
    /// - `issues.json` (all issues, WSJF-sorted, forward-slash paths)
    /// - `issues-fixable.json` (auto_fixable = true)
    /// - `issues-remaining.json` (auto_fixable = false)
    /// - `summary.json`
    /// - `summary.txt`
    pub fn write_all(&self, issues: &[Issue], summary: &ScanSummary) -> anyhow::Result<()> {
        // Sort a clone by priority before writing.
        let mut sorted = issues.to_vec();
        sort_issues(&mut sorted);
        files::write_all_files(&self.dir, &sorted, summary)
    }

    /// Build a [`ScanSummary`] from a slice of [`Issue`]s.
    pub fn build_summary(
        issues: &[Issue],
        project_path: &str,
        scanned_at: &str,
        scanners_run: Vec<String>,
        scan_duration_ms: u64,
    ) -> ScanSummary {
        use std::collections::HashMap;

        let total_issues = issues.len();
        let fixable_count = issues.iter().filter(|i| i.fix.auto_fixable).count();
        let remaining_count = total_issues - fixable_count;

        let mut by_severity: HashMap<String, usize> = HashMap::new();
        let mut by_complexity: HashMap<String, usize> = HashMap::new();
        let mut fix_queue_by_category: HashMap<String, usize> = HashMap::new();

        for issue in issues {
            *by_severity.entry(issue.severity.clone()).or_insert(0) += 1;

            let complexity = match &issue.fix.complexity {
                crate::issue::FixComplexity::Trivial => "trivial",
                crate::issue::FixComplexity::Moderate => "moderate",
                crate::issue::FixComplexity::Complex => "complex",
            };
            *by_complexity.entry(complexity.to_string()).or_insert(0) += 1;

            if issue.fix.auto_fixable {
                let stage_key = match issue.fix.auto_fix_category.as_deref() {
                    Some("formatter") => "formatters",
                    Some("linter") => "linters",
                    Some("security") => "security",
                    Some("ast") => "ast",
                    Some("deps") => "deps",
                    _ => continue,
                };
                *fix_queue_by_category
                    .entry(stage_key.to_string())
                    .or_insert(0) += 1;
            }
        }

        ScanSummary {
            schema_version: "1.0".to_string(),
            scanned_at: scanned_at.to_string(),
            project_path: project_path.to_string(),
            total_issues,
            fixable_count,
            remaining_count,
            by_severity,
            by_complexity,
            scanners_run,
            scan_duration_ms,
            fix_queue_by_category,
        }
    }

    /// Sort issues by WSJF priority score, highest first.
    pub fn sort_by_priority(issues: &mut [Issue]) {
        issues.sort_by_key(|a| std::cmp::Reverse(a.priority_score));
    }
}

/// Sort issues by WSJF priority score, highest first.
///
/// Module-level function for use in the scan CLI pipeline.
pub fn sort_issues(issues: &mut [Issue]) {
    issues.sort_by_key(|a| std::cmp::Reverse(a.priority_score));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_issue(severity: &str, auto_fixable: bool, score: i32) -> Issue {
        use crate::issue::{EvidenceBlock, FixComplexity, FixMetadata, VerificationInfo};

        Issue {
            id: format!("test-{severity}-{score}"),
            rule_id: "test-rule".to_string(),
            severity: severity.to_string(),
            file_path: "src/lib.rs".to_string(),
            line: 1,
            message: "test message".to_string(),
            scanner: "semgrep".to_string(),
            evidence: EvidenceBlock {
                matched_code: String::new(),
                context_before: vec![],
                context_after: vec![],
                enclosing_function: None,
                enclosing_class: None,
                imports: vec![],
            },
            fix: FixMetadata {
                auto_fixable,
                auto_fix_tool: None,
                auto_fix_category: None,
                suggested_replacement: None,
                complexity: FixComplexity::Trivial,
            },
            verification: VerificationInfo {
                rerun_command: "rguard scan .".to_string(),
                success_condition: "no findings".to_string(),
            },
            priority_score: score,
            priority_tier: "medium".to_string(),
            cross_file: false,
        }
    }

    #[test]
    fn sort_by_priority_highest_first() {
        let mut issues = vec![
            make_issue("info", false, 5),
            make_issue("error", true, 90),
            make_issue("warning", false, 20),
        ];
        OutputWriter::sort_by_priority(&mut issues);
        assert_eq!(issues[0].priority_score, 90);
        assert_eq!(issues[1].priority_score, 20);
        assert_eq!(issues[2].priority_score, 5);
    }

    #[test]
    fn build_summary_counts() {
        let issues = vec![
            make_issue("error", true, 90),
            make_issue("warning", false, 20),
            make_issue("info", false, 5),
        ];
        let summary = OutputWriter::build_summary(
            &issues,
            "/project",
            "2026-03-11T00:00:00Z",
            vec!["semgrep".to_string()],
            1000,
        );
        assert_eq!(summary.total_issues, 3);
        assert_eq!(summary.fixable_count, 1);
        assert_eq!(summary.remaining_count, 2);
        assert_eq!(summary.by_severity["error"], 1);
        assert_eq!(summary.by_severity["warning"], 1);
    }

    #[test]
    fn empty_issues_summary() {
        let summary =
            OutputWriter::build_summary(&[], "/project", "2026-03-11T00:00:00Z", vec![], 0);
        assert_eq!(summary.total_issues, 0);
        assert_eq!(summary.fixable_count, 0);
        assert_eq!(summary.remaining_count, 0);
    }

    #[test]
    fn sort_issues_fn_highest_first() {
        let mut issues = vec![
            make_issue("info", false, 5),
            make_issue("error", true, 90),
            make_issue("warning", false, 20),
        ];
        sort_issues(&mut issues);
        assert_eq!(issues[0].priority_score, 90);
        assert_eq!(issues[2].priority_score, 5);
    }
}
