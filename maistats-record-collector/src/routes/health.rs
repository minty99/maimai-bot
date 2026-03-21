use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
struct HealthResponse {
    status: String,
}

#[derive(Serialize)]
struct ReadyResponse {
    status: String,
    database: String,
}

/// GET /health - Simple health check
pub(crate) async fn health() -> impl IntoResponse {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}

/// GET /health/ready - Readiness check with database connectivity
pub(crate) async fn ready(State(state): State<AppState>) -> impl IntoResponse {
    // Try a simple SELECT 1 query to verify database connectivity
    match sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.db_pool)
        .await
    {
        Ok(_) => (
            StatusCode::OK,
            Json(ReadyResponse {
                status: "ready".to_string(),
                database: "ok".to_string(),
            }),
        ),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ReadyResponse {
                status: "not_ready".to_string(),
                database: "error".to_string(),
            }),
        ),
    }
}
