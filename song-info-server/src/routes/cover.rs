use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use tokio::fs;

use crate::error::{AppError, Result};
use crate::state::AppState;

pub(crate) async fn get_cover(
    State(state): State<AppState>,
    Path(image_name): Path<String>,
) -> Result<Response> {
    if image_name.contains("..") || image_name.contains('/') || image_name.contains('\\') {
        return Err(AppError::NotFound("Invalid image name".to_string()));
    }

    let mut file_path = state.song_data_base_path.clone();
    file_path.push("cover");
    file_path.push(&image_name);

    if !file_path.exists() {
        return Err(AppError::NotFound("Cover image not found".to_string()));
    }

    let bytes = fs::read(&file_path)
        .await
        .map_err(|err| AppError::IoError(err.to_string()))?;

    Ok((StatusCode::OK, [(header::CONTENT_TYPE, "image/png")], bytes).into_response())
}
