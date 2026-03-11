//! Output serialization — produces the AI-ready JSON report.
//!
//! # Output Schema
//!
//! ```json
//! {
//!   "version": "0.1.1",
//!   "scanned_at": "2025-01-01T00:00:00Z",
//!   "project_root": "/path/to/project",
//!   "summary": {
//!     "total": 12,
//!     "by_severity": { "error": 2, "warning": 8, "info": 2 },
//!     "by_scanner": { "semgrep": 5, "clippy": 7 },
//!     "autofixable": 3
//!   },
//!   "issues": [ ... ]
//! }
//! ```

use std::{
    collections::HashMap,
    io::{self, Write},
    path::Path,
};

use serde::{Deserialize, Serialize};

use crate::issue::Issue;

/// Top-level report serialized to JSON.
#[derive(Debug, Serialize, Deserialize)]
pub struct ScanReport {
    /// rice-guard schema version (semver).
    pub version: String,
    /// ISO-8601 timestamp when the scan ran.
    pub scanned_at: String,
    /// Absolute path of the scanned project.
    pub project_root: String,
    /// Aggregated counts over all findings.
    pub summary: ScanSummary,
    /// Ordered list of findings (sorted by WSJF score descending).
    pub issues: Vec<Issue>,
}

/// Aggregated statistics over all findings.
#[derive(Debug, Serialize, Deserialize)]
pub struct ScanSummary {
    /// Total number of unique issues.
    pub total: usize,
    /// Count grouped by normalized severity string.
    pub by_severity: HashMap<String, usize>,
    /// Count grouped by scanner name.
    pub by_scanner: HashMap<String, usize>,
    /// Number of issues that have an automated fix available.
    pub autofixable: usize,
}

impl ScanSummary {
    /// Build a [`ScanSummary`] from a slice of [`Issue`]s.
    pub fn from_issues(issues: &[Issue]) -> Self {
        let mut by_severity: HashMap<String, usize> = HashMap::new();
        let mut by_scanner: HashMap<String, usize> = HashMap::new();
        let mut autofixable = 0usize;

        for issue in issues {
            *by_severity.entry(issue.severity.clone()).or_insert(0) += 1;
            *by_scanner.entry(issue.scanner.clone()).or_insert(0) += 1;

            use crate::issue::fix::FixCategory;
            if issue.fix.category == FixCategory::ScannerAutofix
                || issue.fix.category == FixCategory::FormatterApplicable
                || issue.fix.category == FixCategory::PackageManagerFix
            {
                autofixable += 1;
            }
        }

        ScanSummary {
            total: issues.len(),
            by_severity,
            by_scanner,
            autofixable,
        }
    }
}

/// Writes the formatted report.
pub struct OutputWriter;

impl OutputWriter {
    /// Serialize `report` as pretty-printed JSON and write to `writer`.
    pub fn write_json(report: &ScanReport, writer: &mut dyn Write) -> io::Result<()> {
        let json = serde_json::to_string_pretty(report).map_err(io::Error::other)?;
        writer.write_all(json.as_bytes())?;
        writer.write_all(b"\n")
    }

    /// Build a [`ScanReport`] from `issues`, sorted by WSJF score descending.
    pub fn build_report(
        issues: Vec<Issue>,
        project_root: &Path,
        version: &str,
        scanned_at: &str,
    ) -> ScanReport {
        let mut sorted = issues;
        sorted.sort_by(|a, b| {
            b.priority
                .wsjf_score
                .partial_cmp(&a.priority.wsjf_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let summary = ScanSummary::from_issues(&sorted);

        ScanReport {
            version: version.to_string(),
            scanned_at: scanned_at.to_string(),
            project_root: project_root.to_string_lossy().to_string(),
            summary,
            issues: sorted,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_empty_report() -> ScanReport {
        OutputWriter::build_report(vec![], Path::new("/tmp/project"), "0.1.1", "2025-01-01")
    }

    #[test]
    fn empty_report_serializes() {
        let report = make_empty_report();
        let mut buf = Vec::new();
        OutputWriter::write_json(&report, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("\"total\": 0"));
    }

    #[test]
    fn empty_summary_has_zero_counts() {
        let summary = ScanSummary::from_issues(&[]);
        assert_eq!(summary.total, 0);
        assert_eq!(summary.autofixable, 0);
    }
}
