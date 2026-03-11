use serde::{Deserialize, Serialize};

/// Stub issue type — fully implemented in Phase 3.
///
/// Phase 3 will add evidence fields, priority scoring, fix metadata,
/// and SARIF serialization. This stub exists so downstream crates
/// can import `rice_guard_core::issue::Issue` today.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    /// Stable content-hash fingerprint ID (rule_id + normalized_path + matched_code).
    pub id: String,

    /// Rule identifier from the scanner (e.g., "semgrep.python.security.sql-injection").
    pub rule_id: String,

    /// Severity level: `"error"`, `"warning"`, or `"info"`.
    pub severity: String,

    /// Relative file path (normalized to forward slashes).
    pub file_path: String,

    /// 1-based line number of the finding.
    pub line: u32,

    /// Human-readable message describing the issue.
    pub message: String,
}
