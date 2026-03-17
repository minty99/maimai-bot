use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::Serialize;

use crate::http_client::{MAIMAI_UNAVAILABLE_MESSAGE, is_maintenance_error};

#[derive(Debug)]
pub(crate) enum AppError {
    DatabaseError(String),
    NotFound(String),
    InternalError(String),
    BadRequest(String),
    Maintenance(String),
}

#[derive(Serialize)]
struct ErrorResponse {
    message: String,
    code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    maintenance: Option<bool>,
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, message, code, maintenance) = match self {
            AppError::DatabaseError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                msg,
                "DATABASE_ERROR",
                None,
            ),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg, "NOT_FOUND", None),
            AppError::InternalError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                msg,
                "INTERNAL_ERROR",
                None,
            ),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg, "BAD_REQUEST", None),
            AppError::Maintenance(msg) => (
                StatusCode::SERVICE_UNAVAILABLE,
                msg,
                "MAINTENANCE",
                Some(true),
            ),
        };
        (
            status,
            Json(ErrorResponse {
                message,
                code: code.to_string(),
                maintenance,
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

pub(crate) type Result<T> = std::result::Result<T, AppError>;

pub(crate) fn app_error_from_maimai(err: eyre::Error) -> AppError {
    if is_maintenance_error(&err) {
        return AppError::Maintenance(MAIMAI_UNAVAILABLE_MESSAGE.to_string());
    }

    AppError::InternalError(err.to_string())
}
