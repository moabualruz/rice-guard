use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
pub struct ScanRequest {
    pub path: Option<String>,
    pub mode: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct FixRequest {
    pub path: Option<String>,
    pub dry_run: Option<bool>,
    pub unsafe_fixes: Option<bool>,
    pub category: Option<String>,
    pub issue_id: Option<String>,
}
