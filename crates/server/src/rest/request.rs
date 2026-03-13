use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
pub struct ScanRequest {
    pub path: Option<String>,
    pub mode: Option<String>,
    /// When `true`, bypass ignore filtering (.rgignore / .gitignore).
    /// Default: `false` (ignore filtering is applied).
    pub include_ignored: Option<bool>,
}

#[derive(Debug, Deserialize, Default)]
pub struct FixRequest {
    pub path: Option<String>,
    pub dry_run: Option<bool>,
    pub unsafe_fixes: Option<bool>,
    pub category: Option<String>,
    pub issue_id: Option<String>,
    /// When `true`, bypass ignore filtering (.rgignore / .gitignore).
    /// Default: `false` (ignore filtering is applied).
    pub include_ignored: Option<bool>,
}
