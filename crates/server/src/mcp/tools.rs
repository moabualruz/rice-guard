// schemars 1.x — the server crate overrides the workspace 0.8 pin to use 1.x,
// which is required by rmcp's Parameters<T> trait bound.
use schemars::JsonSchema;
use serde::Deserialize;

/// Parameters for the `scan_project` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ScanProjectParams {
    /// Path to the project directory to scan.
    pub path: String,
    /// Scan mode: "full" | "quick" | "security". Default: "full".
    #[serde(default = "default_full")]
    pub mode: Option<String>,
    /// When true, bypass ignore filtering (.rgignore / .gitignore). Default: false.
    #[serde(default)]
    pub include_ignored: Option<bool>,
}

/// Parameters for the `get_issues` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetIssuesParams {
    /// Path to the project directory (reads latest issues.json from reports/).
    pub path: String,
    /// Optional severity filter: "error" | "warning" | "info".
    #[serde(default)]
    pub severity: Option<String>,
    /// Maximum number of issues to return.
    #[serde(default)]
    pub limit: Option<usize>,
}

/// Parameters for the `get_issue` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetIssueParams {
    /// Path to the project directory.
    pub path: String,
    /// The content-hash fingerprint ID of the issue (from issues.json).
    pub id: String,
}

/// Parameters for the `fix_all` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct FixAllParams {
    /// Path to the project directory.
    pub path: String,
    /// Fixer category filter: "formatters" | "linters" | "security" | "ast" | "deps" | "imports".
    #[serde(default)]
    pub category: Option<String>,
    /// Include unsafe linter fixes. Default: false.
    #[serde(default)]
    pub unsafe_fixes: bool,
    /// When true, bypass ignore filtering (.rgignore / .gitignore). Default: false.
    #[serde(default)]
    pub include_ignored: Option<bool>,
}

/// Parameters for the `fix_issue` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct FixIssueParams {
    /// Path to the project directory.
    pub path: String,
    /// The content-hash fingerprint ID of the issue to fix.
    pub id: String,
}

/// Parameters for the `fix_preview` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct FixPreviewParams {
    /// Path to the project directory.
    pub path: String,
    /// Fixer category to preview. If None, previews all.
    #[serde(default)]
    pub category: Option<String>,
}

/// Parameters for the `get_fix_report` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetFixReportParams {
    /// Path to the project directory (reads latest fix-report.json from reports/).
    pub path: String,
}

/// Parameters for the `verify_fix` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct VerifyFixParams {
    /// Path to the project directory.
    pub path: String,
    /// The content-hash fingerprint ID of the issue that was fixed.
    pub id: String,
}

/// Parameters for the `get_remaining_issues` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetRemainingIssuesParams {
    /// Path to the project directory.
    pub path: String,
}

fn default_full() -> Option<String> {
    Some("full".to_string())
}
