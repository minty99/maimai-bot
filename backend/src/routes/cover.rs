use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use tokio::fs;

use crate::state::AppState;

pub async fn get_cover(State(state): State<AppState>, Path(image_name): Path<String>) -> Response {
    if image_name.contains("..") || image_name.contains('/') || image_name.contains('\\') {
        return (StatusCode::BAD_REQUEST, "Invalid image name").into_response();
    }

    let mut file_path = state.song_data_base_path.clone();
    file_path.push("cover");
    file_path.push(&image_name);

    if !file_path.exists() {
        return (StatusCode::NOT_FOUND, "Cover image not found").into_response();
    }

    match fs::read(&file_path).await {
        Ok(bytes) => {
            // Note: song cover assets are stored as PNG bytes (upstream-style).
            let content_type = "image/png";

            (
                StatusCode::OK,
                [(axum::http::header::CONTENT_TYPE, content_type)],
                bytes,
            )
                .into_response()
        }
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Failed to read file").into_response(),
    }
}
