use schemars::JsonSchema;
use serde::Deserialize;

fn default_scan_mode() -> String {
    "full".to_string()
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ScanProjectParams {
    /// Path to the project directory to scan.
    pub path: String,
    /// Scan mode: "full" | "quick" | "security". Default: "full".
    #[serde(default = "default_scan_mode")]
    pub mode: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetIssuesParams {
    /// Optional severity filter: "error" | "warning" | "info".
    pub severity: Option<String>,
    /// Maximum number of issues to return.
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetIssueParams {
    /// The content-hash fingerprint ID of the issue (from issues.json).
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FixAllParams {
    /// Path to the project directory.
    pub path: String,
    /// Fixer category filter: "formatters" | "linters" | "security" | "ast" | "deps" | "imports".
    pub category: Option<String>,
    /// Include unsafe linter fixes. Default: false.
    #[serde(default)]
    pub unsafe_fixes: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FixIssueParams {
    /// The content-hash fingerprint ID of the issue to fix.
    pub id: String,
    /// Path to the project directory.
    pub path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FixPreviewParams {
    /// Path to the project directory.
    pub path: String,
    /// Fixer category to preview. If None, previews all.
    pub category: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetFixReportParams {
    /// Path to the project directory (reads latest fix-report.json from reports/).
    pub path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VerifyFixParams {
    /// The content-hash fingerprint ID of the issue that was fixed.
    pub id: String,
    /// Path to the project directory.
    pub path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetRemainingIssuesParams {
    /// Path to the project directory.
    pub path: String,
}
