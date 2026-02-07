use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;
use std::sync::atomic::Ordering;

use crate::state::AppState;

#[derive(Serialize)]
struct HealthResponse {
    status: String,
}

#[derive(Serialize)]
struct ReadyResponse {
    status: String,
    song_data: String,
}

pub async fn health() -> impl IntoResponse {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}

pub async fn ready(State(state): State<AppState>) -> impl IntoResponse {
    let song_data_available =
        state.song_data.read().is_ok() && state.song_data_loaded.load(Ordering::Relaxed);

    if song_data_available {
        (
            StatusCode::OK,
            Json(ReadyResponse {
                status: "ready".to_string(),
                song_data: "ok".to_string(),
            }),
        )
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ReadyResponse {
                status: "not_ready".to_string(),
                song_data: "missing".to_string(),
            }),
        )
    }
}
