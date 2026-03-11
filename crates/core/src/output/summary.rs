//! `ScanSummary` — aggregated statistics from a completed scan.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::issue::{FixComplexity, Issue};

/// Aggregated statistics over all findings from a scan.
///
/// Serialized to `summary.json` in the output directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanSummary {
    /// ISO-8601 timestamp when the scan started.
    pub scanned_at: String,

    /// Absolute path of the scanned project root.
    pub project_path: String,

    /// Total number of unique issues found.
    pub total_issues: usize,

    /// Number of issues where `fix.auto_fixable` is true.
    pub fixable_count: usize,

    /// Number of issues where `fix.auto_fixable` is false (need manual/AI fix).
    pub remaining_count: usize,

    /// Count of issues grouped by normalized severity (`"error"`, `"warning"`, `"info"`).
    pub by_severity: HashMap<String, usize>,

    /// Count of issues grouped by fix complexity (`"trivial"`, `"moderate"`, `"complex"`).
    pub by_complexity: HashMap<String, usize>,

    /// Names of scanners that ran in this scan.
    pub scanners_run: Vec<String>,

    /// Total wall-clock duration of the scan in milliseconds.
    pub scan_duration_ms: u64,
}

impl ScanSummary {
    /// Construct a [`ScanSummary`] from a slice of [`Issue`]s.
    ///
    /// Uses `chrono::Utc::now()` for `scanned_at`.
    pub fn from_issues(
        issues: &[Issue],
        scanners_run: Vec<String>,
        project_path: &str,
        duration_ms: u64,
    ) -> Self {
        let total_issues = issues.len();
        let fixable_count = issues.iter().filter(|i| i.fix.auto_fixable).count();
        let remaining_count = total_issues - fixable_count;

        let mut by_severity: HashMap<String, usize> = HashMap::new();
        let mut by_complexity: HashMap<String, usize> = HashMap::new();

        for issue in issues {
            *by_severity.entry(issue.severity.clone()).or_insert(0) += 1;

            let complexity = match &issue.fix.complexity {
                FixComplexity::Trivial => "trivial",
                FixComplexity::Moderate => "moderate",
                FixComplexity::Complex => "complex",
            };
            *by_complexity.entry(complexity.to_string()).or_insert(0) += 1;
        }

        let scanned_at = chrono::Utc::now().to_rfc3339();

        ScanSummary {
            scanned_at,
            project_path: project_path.to_string(),
            total_issues,
            fixable_count,
            remaining_count,
            by_severity,
            by_complexity,
            scanners_run,
            scan_duration_ms: duration_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_summary_serializes_to_json() {
        let summary = ScanSummary {
            scanned_at: "2026-03-11T00:00:00Z".to_string(),
            project_path: "/project".to_string(),
            total_issues: 5,
            fixable_count: 2,
            remaining_count: 3,
            by_severity: HashMap::from([
                ("error".to_string(), 1usize),
                ("warning".to_string(), 4usize),
            ]),
            by_complexity: HashMap::from([("trivial".to_string(), 2usize)]),
            scanners_run: vec!["semgrep".to_string()],
            scan_duration_ms: 500,
        };

        let json = serde_json::to_string(&summary).expect("serialize");
        assert!(json.contains("total_issues"));
        assert!(json.contains("fixable_count"));
        assert!(json.contains("scan_duration_ms"));
    }

    #[test]
    fn scan_summary_fixable_plus_remaining_equals_total() {
        let summary = ScanSummary {
            scanned_at: String::new(),
            project_path: String::new(),
            total_issues: 10,
            fixable_count: 3,
            remaining_count: 7,
            by_severity: HashMap::new(),
            by_complexity: HashMap::new(),
            scanners_run: vec![],
            scan_duration_ms: 0,
        };
        assert_eq!(
            summary.fixable_count + summary.remaining_count,
            summary.total_issues
        );
    }

    #[test]
    fn from_issues_counts_correctly() {
        use crate::issue::{EvidenceBlock, FixMetadata, VerificationInfo};

        let make = |auto_fixable: bool| Issue {
            id: "test".to_string(),
            rule_id: "rule".to_string(),
            severity: "warning".to_string(),
            file_path: "src/lib.rs".to_string(),
            line: 1,
            message: "msg".to_string(),
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
                rerun_command: "rice-guard scan .".to_string(),
                success_condition: "no findings".to_string(),
            },
            priority_score: 20,
            cross_file: false,
        };

        let issues = vec![make(true), make(true), make(false)];
        let summary = ScanSummary::from_issues(&issues, vec![], "/project", 100);
        assert_eq!(summary.total_issues, 3);
        assert_eq!(summary.fixable_count, 2);
        assert_eq!(summary.remaining_count, 1);
        assert!(!summary.scanned_at.is_empty(), "scanned_at must be set");
    }
}
