pub mod error;
pub mod handlers;
pub mod request;
pub mod state;

use std::path::PathBuf;
use std::time::Duration;

use axum::Router;
use axum::routing::{get, post};
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

    println!("rice-guard serving on http://{addr}");
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

    fn test_state() -> AppState {
        AppState {
            working_dir: PathBuf::from("."),
        }
    }

    #[tokio::test]
    async fn health_returns_200() {
        let app = build_router(test_state());
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
    #[ignore]
    async fn scan_handler_returns_summary() {
        todo!()
    }

    #[tokio::test]
    #[ignore]
    async fn issues_handler_returns_vec() {
        todo!()
    }

    #[tokio::test]
    #[ignore]
    async fn issue_by_id_returns_single_or_404() {
        todo!()
    }

    #[tokio::test]
    #[ignore]
    async fn fix_handler_returns_fix_report() {
        todo!()
    }

    #[tokio::test]
    #[ignore]
    async fn status_handler_returns_summary() {
        todo!()
    }

    #[tokio::test]
    #[ignore]
    async fn fix_dry_run_does_not_modify_files() {
        todo!()
    }

    #[tokio::test]
    #[ignore]
    async fn fix_category_filter_runs_only_that_stage() {
        todo!()
    }
}
