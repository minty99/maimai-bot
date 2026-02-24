use axum::{extract::State, Json};
use eyre::WrapErr;

use crate::error::Result;
use crate::http_client::is_maintenance_window_now;
use crate::state::AppState;
use maimai_parsers::parse_rating_target_music_html;
use models::ParsedRatingTargets;

pub(crate) async fn get_rating_targets(
    State(state): State<AppState>,
) -> Result<Json<ParsedRatingTargets>> {
    if is_maintenance_window_now() {
        return Err(crate::error::AppError::Maintenance(
            "maimai DX NET maintenance window (04:00-07:00 local time)".to_string(),
        ));
    }

    let mut client = state
        .maimai_client()
        .wrap_err("create HTTP client")
        .map_err(|e| crate::error::AppError::InternalError(e.to_string()))?;

    client
        .ensure_logged_in()
        .await
        .wrap_err("ensure logged in")
        .map_err(|e| crate::error::AppError::InternalError(e.to_string()))?;

    let url = reqwest::Url::parse("https://maimaidx-eng.com/maimai-mobile/home/ratingTargetMusic/")
        .wrap_err("parse ratingTargetMusic url")
        .map_err(|e| crate::error::AppError::InternalError(e.to_string()))?;

    let bytes = client
        .get_bytes(&url)
        .await
        .wrap_err("fetch ratingTargetMusic url")
        .map_err(|e| crate::error::AppError::HttpClientError(e.to_string()))?;
    let html = String::from_utf8(bytes)
        .wrap_err("ratingTargetMusic response is not utf-8")
        .map_err(|e| crate::error::AppError::InternalError(e.to_string()))?;

    let parsed = parse_rating_target_music_html(&html)
        .wrap_err("parse ratingTargetMusic html")
        .map_err(|e| crate::error::AppError::InternalError(e.to_string()))?;

    Ok(Json(parsed))
}
