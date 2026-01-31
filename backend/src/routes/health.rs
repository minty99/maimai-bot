use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
pub struct HealthResponse {
    status: String,
}

#[derive(Serialize)]
pub struct ReadyResponse {
    status: String,
    database: String,
}

/// GET /health - Simple health check
pub async fn health() -> impl IntoResponse {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}

/// GET /health/ready - Readiness check with database connectivity
pub async fn ready(State(state): State<AppState>) -> impl IntoResponse {
    // Try a simple SELECT 1 query to verify database connectivity
    match sqlx::query_scalar::<_, i32>("SELECT 1").fetch_one(&state.db_pool).await {
        Ok(_) => (
            StatusCode::OK,
            Json(ReadyResponse {
                status: "ready".to_string(),
                database: "ok".to_string(),
            }),
        ),
        Err(e) => {
            tracing::error!("Database health check failed: {}", e);
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ReadyResponse {
                    status: "not_ready".to_string(),
                    database: "error".to_string(),
                }),
            )
        }
    }
}
