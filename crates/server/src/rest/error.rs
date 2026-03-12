use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

pub struct ApiError {
    pub status: StatusCode,
    pub message: String,
    pub code: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        todo!()
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
