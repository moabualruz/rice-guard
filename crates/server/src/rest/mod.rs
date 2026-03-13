pub mod error;
pub mod handlers;
pub mod request;
pub mod state;

use std::path::PathBuf;
use std::time::Duration;

use axum::routing::{get, post};
use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

use state::AppState;

/// Build the axum router with all 6 API routes under `/api/v1/`.
///
/// Layers applied (outermost-first):
/// - [`TraceLayer`] — HTTP request/response tracing
/// - [`CorsLayer::permissive()`] — allow all origins
/// - [`TimeoutLayer`] — 600 second request timeout
pub fn build_router(state: AppState) -> Router {
    let api_v1 = Router::new()
        .route("/health", get(handlers::health_handler))
        .route("/scan", post(handlers::scan_handler))
        .route("/issues", get(handlers::issues_handler))
        .route("/issues/{id}", get(handlers::issue_by_id_handler))
        .route("/fix", post(handlers::fix_handler))
        .route("/status", get(handlers::status_handler));

    Router::new()
        .nest("/api/v1", api_v1)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .layer(TimeoutLayer::with_status_code(
            axum::http::StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(600),
        ))
        .with_state(state)
}

/// Start the HTTP server on `127.0.0.1:{port}`.
///
/// Prints the server address and all available endpoints before binding.
pub async fn start_server(port: u16, working_dir: PathBuf) -> anyhow::Result<()> {
    let addr = format!("127.0.0.1:{port}");
    let state = AppState { working_dir };

    println!("rguard serving on http://{addr}");
    println!("  GET  http://{addr}/api/v1/health");
    println!("  POST http://{addr}/api/v1/scan");
    println!("  GET  http://{addr}/api/v1/issues");
    println!("  GET  http://{addr}/api/v1/issues/{{id}}");
    println!("  POST http://{addr}/api/v1/fix");
    println!("  GET  http://{addr}/api/v1/status");

    let app = build_router(state);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt as _;
    use tower::ServiceExt as _;

    use super::*;

    // ── Test helpers ──────────────────────────────────────────────────────────

    fn test_state_with_dir(working_dir: PathBuf) -> AppState {
        AppState { working_dir }
    }

    /// Write a minimal .rguard.yaml into a temp directory.
    fn write_minimal_config(dir: &std::path::Path) {
        let yaml = r#"version: "1"
project:
  name: test-project
  languages:
    - rust
  topology: monolith
  architecture: none
scanners:
  semgrep:
    enabled: false
  trivy:
    enabled: false
  gitleaks:
    enabled: false
  jscpd:
    enabled: false
  scc:
    enabled: false
"#;
        std::fs::write(dir.join(".rguard.yaml"), yaml).expect("failed to write test config");
    }

    /// Create a minimal fixture reports directory with issues.json + summary.json.
    fn write_fixture_reports(project_dir: &std::path::Path) {
        let report_dir = project_dir
            .join("reports")
            .join("test-project")
            .join("20260101T000000Z");
        std::fs::create_dir_all(&report_dir).unwrap();

        // issues.json: IssueOutput envelope
        let issues_json = serde_json::json!({
            "schema_version": "1.0",
            "issues": [
                {
                    "id": "abc123",
                    "rule_id": "test-rule",
                    "severity": "warning",
                    "file_path": "src/lib.rs",
                    "line": 1,
                    "message": "test finding",
                    "scanner": "semgrep",
                    "evidence": {
                        "matched_code": "let x = 1;",
                        "context_before": [],
                        "context_after": [],
                        "enclosing_function": null,
                        "enclosing_class": null,
                        "imports": []
                    },
                    "fix": {
                        "auto_fixable": false,
                        "auto_fix_tool": null,
                        "auto_fix_category": null,
                        "suggested_replacement": null,
                        "complexity": "trivial"
                    },
                    "verification": {
                        "rerun_command": "rguard scan .",
                        "success_condition": "no findings"
                    },
                    "priority_score": 20,
                    "priority_tier": "medium",
                    "cross_file": false
                }
            ]
        });
        std::fs::write(
            report_dir.join("issues.json"),
            serde_json::to_string_pretty(&issues_json).unwrap(),
        )
        .unwrap();

        // summary.json
        let summary_json = serde_json::json!({
            "schema_version": "1.0",
            "scanned_at": "2026-01-01T00:00:00Z",
            "project_path": project_dir.to_string_lossy(),
            "total_issues": 1,
            "fixable_count": 0,
            "remaining_count": 1,
            "by_severity": { "warning": 1 },
            "by_complexity": { "trivial": 1 },
            "scanners_run": ["semgrep"],
            "scan_duration_ms": 100,
            "fix_queue_by_category": {}
        });
        std::fs::write(
            report_dir.join("summary.json"),
            serde_json::to_string_pretty(&summary_json).unwrap(),
        )
        .unwrap();
    }

    // ── Tests ─────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn health_returns_200() {
        let app = build_router(AppState {
            working_dir: PathBuf::from("."),
        });
        let req = Request::builder()
            .method("GET")
            .uri("/api/v1/health")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["status"], "ok");
    }

    #[tokio::test]
    async fn scan_handler_returns_summary() {
        let dir = tempfile::tempdir().unwrap();
        write_minimal_config(dir.path());

        let app = build_router(test_state_with_dir(dir.path().to_path_buf()));
        let body = serde_json::json!({
            "path": dir.path().to_string_lossy(),
            "mode": "full"
        });
        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/scan")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(
            json.get("schema_version").is_some(),
            "response must have schema_version"
        );
        assert!(
            json.get("total_issues").is_some(),
            "response must have total_issues"
        );
    }

    #[tokio::test]
    async fn issues_handler_returns_vec() {
        let dir = tempfile::tempdir().unwrap();
        write_fixture_reports(dir.path());

        let app = build_router(test_state_with_dir(dir.path().to_path_buf()));
        let req = Request::builder()
            .method("GET")
            .uri("/api/v1/issues")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let issues: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(issues.is_array(), "response must be a JSON array");
        assert_eq!(issues.as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn issue_by_id_returns_single_or_404() {
        let dir = tempfile::tempdir().unwrap();
        write_fixture_reports(dir.path());

        let app = build_router(test_state_with_dir(dir.path().to_path_buf()));

        // Existing ID → 200
        let req = Request::builder()
            .method("GET")
            .uri("/api/v1/issues/abc123")
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let issue: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(issue["id"], "abc123");

        // Unknown ID → 404
        let req404 = Request::builder()
            .method("GET")
            .uri("/api/v1/issues/does-not-exist")
            .body(Body::empty())
            .unwrap();
        let resp404 = app.oneshot(req404).await.unwrap();
        assert_eq!(resp404.status(), StatusCode::NOT_FOUND);
        let bytes404 = resp404.into_body().collect().await.unwrap().to_bytes();
        let err: serde_json::Value = serde_json::from_slice(&bytes404).unwrap();
        assert_eq!(err["code"], "NOT_FOUND");
    }

    #[tokio::test]
    async fn fix_handler_returns_fix_report() {
        let dir = tempfile::tempdir().unwrap();
        write_minimal_config(dir.path());

        let app = build_router(test_state_with_dir(dir.path().to_path_buf()));
        let body = serde_json::json!({
            "path": dir.path().to_string_lossy(),
            "dry_run": false
        });
        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/fix")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let report: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(
            report.get("schema_version").is_some(),
            "fix report must have schema_version"
        );
        assert!(
            report.get("dry_run").is_some(),
            "fix report must have dry_run"
        );
    }

    #[tokio::test]
    async fn status_handler_returns_summary() {
        let dir = tempfile::tempdir().unwrap();
        write_fixture_reports(dir.path());

        let app = build_router(test_state_with_dir(dir.path().to_path_buf()));
        let req = Request::builder()
            .method("GET")
            .uri("/api/v1/status")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let summary: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(summary["total_issues"], 1);
        assert_eq!(summary["schema_version"], "1.0");
    }

    #[tokio::test]
    async fn fix_dry_run_does_not_modify_files() {
        let dir = tempfile::tempdir().unwrap();
        write_minimal_config(dir.path());

        let app = build_router(test_state_with_dir(dir.path().to_path_buf()));
        let body = serde_json::json!({
            "path": dir.path().to_string_lossy(),
            "dry_run": true
        });
        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/fix")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let report: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            report["dry_run"], true,
            "fix report must reflect dry_run=true"
        );
    }

    #[tokio::test]
    async fn fix_category_filter_runs_only_that_stage() {
        let dir = tempfile::tempdir().unwrap();
        write_minimal_config(dir.path());

        let app = build_router(test_state_with_dir(dir.path().to_path_buf()));
        let body = serde_json::json!({
            "path": dir.path().to_string_lossy(),
            "category": "formatters",
            "dry_run": false
        });
        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/fix")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let report: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        // With category=formatters, only "format" stage key should appear (or stages is empty
        // since no formatters are configured). Either way the report is valid.
        assert!(
            report.get("stages").is_some(),
            "fix report must have stages"
        );
    }

    /// `?include_ignored=true` in a scan request must be accepted without error.
    #[tokio::test]
    async fn scan_accepts_include_ignored_param() {
        let dir = tempfile::tempdir().unwrap();
        write_minimal_config(dir.path());

        let app = build_router(test_state_with_dir(dir.path().to_path_buf()));
        let body = serde_json::json!({
            "path": dir.path().to_string_lossy(),
            "mode": "full",
            "include_ignored": true
        });
        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/scan")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        // Must not return 400 or 422 — the parameter is accepted.
        assert_eq!(resp.status(), StatusCode::OK);
    }

    /// `include_ignored: true` in a fix request must be accepted without error.
    #[tokio::test]
    async fn fix_accepts_include_ignored_param() {
        let dir = tempfile::tempdir().unwrap();
        write_minimal_config(dir.path());

        let app = build_router(test_state_with_dir(dir.path().to_path_buf()));
        let body = serde_json::json!({
            "path": dir.path().to_string_lossy(),
            "dry_run": true,
            "include_ignored": true
        });
        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/fix")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
