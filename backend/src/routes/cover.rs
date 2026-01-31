use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::path::PathBuf;
use tokio::fs;

use crate::state::AppState;

pub async fn get_cover(
    State(state): State<AppState>,
    Path(image_name): Path<String>,
) -> Response {
    if image_name.contains("..") || image_name.contains('/') || image_name.contains('\\') {
        return (StatusCode::BAD_REQUEST, "Invalid image name").into_response();
    }
    
    let mut file_path = PathBuf::from(&state.fetched_data_path);
    file_path.push("img");
    file_path.push("cover-m");
    file_path.push(&image_name);
    
    if !file_path.exists() {
        return (StatusCode::NOT_FOUND, "Cover image not found").into_response();
    }

    match fs::read(&file_path).await {
        Ok(bytes) => {
            let content_type = if image_name.ends_with(".png") {
                "image/png"
            } else if image_name.ends_with(".jpg") || image_name.ends_with(".jpeg") {
                "image/jpeg"
            } else if image_name.ends_with(".webp") {
                "image/webp"
            } else {
                "application/octet-stream"
            };

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
