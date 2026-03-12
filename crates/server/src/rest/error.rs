use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

/// API error type for all REST endpoints.
///
/// Produces a JSON body `{"error": "...", "code": "..."}` with the
/// corresponding HTTP status code.
pub struct ApiError {
    pub status: StatusCode,
    pub message: String,
    pub code: String,
}

impl ApiError {
    /// Bad request (400) with a custom code and message.
    pub fn bad_request(code: &str, msg: &str) -> Self {
        ApiError {
            status: StatusCode::BAD_REQUEST,
            message: msg.to_string(),
            code: code.to_string(),
        }
    }

    /// Not found (404) with a descriptive message.
    pub fn not_found(msg: &str) -> Self {
        ApiError {
            status: StatusCode::NOT_FOUND,
            message: msg.to_string(),
            code: "NOT_FOUND".to_string(),
        }
    }

    /// Internal server error (500) with a descriptive message.
    pub fn internal(msg: &str) -> Self {
        ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: msg.to_string(),
            code: "INTERNAL_ERROR".to_string(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = Json(json!({
            "error": self.message,
            "code": self.code,
        }));
        (self.status, body).into_response()
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(e: anyhow::Error) -> Self {
        ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: e.to_string(),
            code: "INTERNAL_ERROR".to_string(),
        }
    }
}
