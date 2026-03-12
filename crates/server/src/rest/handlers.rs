use axum::extract::{Path, State};
use axum::Json;
use serde_json::{json, Value};

use super::error::ApiError;
use super::request::{FixRequest, ScanRequest};
use super::state::AppState;

/// GET /api/v1/health — always returns 200 {"status": "ok"}.
pub async fn health_handler() -> Json<Value> {
    Json(json!({"status": "ok"}))
}

/// POST /api/v1/scan — run the scanner engine and return a ScanSummary.
pub async fn scan_handler(
    State(_state): State<AppState>,
    Json(_req): Json<ScanRequest>,
) -> Result<Json<Value>, ApiError> {
    todo!()
}

/// GET /api/v1/issues — return all issues from the latest scan.
pub async fn issues_handler(
    State(_state): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    todo!()
}

/// GET /api/v1/issues/{id} — return a single issue by fingerprint ID or 404.
pub async fn issue_by_id_handler(
    State(_state): State<AppState>,
    Path(_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    todo!()
}

/// POST /api/v1/fix — run deterministic fixers and return a FixReport.
pub async fn fix_handler(
    State(_state): State<AppState>,
    Json(_req): Json<FixRequest>,
) -> Result<Json<Value>, ApiError> {
    todo!()
}

/// GET /api/v1/status — return the ScanSummary from the latest scan.
pub async fn status_handler(
    State(_state): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    todo!()
}
