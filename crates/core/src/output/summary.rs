//! `ScanSummary` — aggregated statistics from a completed scan.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

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
}
