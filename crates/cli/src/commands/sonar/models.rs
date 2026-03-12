/// SonarQube API response structs and Generic Issue Data output format.
use serde::{Deserialize, Serialize};

// ── Generic Issue Data (output format for SonarQube import) ──────────────────

/// Top-level wrapper for SonarQube Generic Issue Data JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SonarGenericIssuesFile {
    pub issues: Vec<SonarGenericIssue>,
}

/// A single issue in SonarQube Generic Issue Data format.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SonarGenericIssue {
    pub engine_id: String,
    pub rule_id: String,
    pub severity: String,
    #[serde(rename = "type")]
    pub issue_type: String,
    pub primary_location: PrimaryLocation,
}

/// Primary location of a SonarQube generic issue.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrimaryLocation {
    pub message: String,
    pub file_path: String,
    pub text_range: TextRange,
}

/// Line/column range for an issue location.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextRange {
    pub start_line: u32,
    pub end_line: u32,
    pub start_column: u32,
    pub end_column: u32,
}

// ── SonarQube API response structs ───────────────────────────────────────────

/// Wrapper for SonarQube API responses that may contain errors even on HTTP 200.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SonarResponse<T> {
    #[serde(flatten)]
    pub data: Option<T>,
    pub errors: Option<Vec<SonarError>>,
}

/// An error entry returned inside a SonarQube 200 response body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SonarError {
    pub msg: String,
}

/// Response from `GET /api/projects/search`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectsSearchResponse {
    pub components: Vec<SonarProject>,
}

/// A single project entry in a SonarQube projects search response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SonarProject {
    pub key: String,
    pub name: String,
}

/// Response from `GET /api/qualitygates/project_status`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QualityGateStatus {
    pub project_status: ProjectStatus,
}

/// Project status block inside a quality gate response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectStatus {
    pub status: String,
    pub conditions: Vec<QualityGateCondition>,
}

/// A single quality gate condition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QualityGateCondition {
    pub status: String,
    pub metric_key: String,
    pub comparator: String,
    pub actual_value: Option<String>,
    pub error_threshold: Option<String>,
}

/// Response from `GET /api/measures/component`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeasuresResponse {
    pub component: ComponentMeasures,
}

/// Component measures block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentMeasures {
    pub measures: Vec<Measure>,
}

/// A single measure (metric + value).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Measure {
    pub metric: String,
    pub value: Option<String>,
}

/// Response from `GET /api/issues/search`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SonarIssuesSearchResponse {
    pub issues: Vec<SonarIssue>,
    pub total: u32,
}

/// A single issue from SonarQube issues search.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SonarIssue {
    pub key: String,
    pub rule: String,
    pub severity: String,
    pub component: String,
    pub message: String,
    pub r#type: Option<String>,
}
