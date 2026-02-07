use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;

#[derive(Debug)]
pub(crate) enum AppError {
    NotFound(String),
    IoError(String),
    JsonError(String),
}

#[derive(Serialize)]
struct ErrorResponse {
    message: String,
    code: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, message, code) = match self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg, "NOT_FOUND"),
            AppError::IoError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg, "IO_ERROR"),
            AppError::JsonError(msg) => (StatusCode::BAD_REQUEST, msg, "JSON_ERROR"),
        };

        (
            status,
            Json(ErrorResponse {
                message,
                code: code.to_string(),
            }),
        )
            .into_response()
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::IoError(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::JsonError(e.to_string())
    }
}

pub(crate) type Result<T> = std::result::Result<T, AppError>;
