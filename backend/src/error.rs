use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;

#[derive(Debug)]
pub enum AppError {
    DatabaseError(String),
    HttpClientError(String),
    NotFound(String),
    InternalError(String),
    BadRequest(String),
}

#[derive(Serialize)]
struct ErrorResponse {
    message: String,
    code: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, message, code) = match self {
            AppError::DatabaseError(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, msg, "DATABASE_ERROR")
            }
            AppError::HttpClientError(msg) => (StatusCode::BAD_GATEWAY, msg, "HTTP_CLIENT_ERROR"),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg, "NOT_FOUND"),
            AppError::InternalError(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, msg, "INTERNAL_ERROR")
            }
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg, "BAD_REQUEST"),
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

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        AppError::DatabaseError(e.to_string())
    }
}

impl From<eyre::Error> for AppError {
    fn from(e: eyre::Error) -> Self {
        AppError::InternalError(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
