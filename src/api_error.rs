use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

#[derive(Debug)]
pub enum ApiError {
    JobQueueClosed,
    InternalError(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::JobQueueClosed => (
                StatusCode::SERVICE_UNAVAILABLE,
                "job queue closed or unavailable",
            )
                .into_response(),
            ApiError::InternalError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("internal error: {msg}"),
            )
                .into_response(),
        }
    }
}
