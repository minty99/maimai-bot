use axum::{extract::State, http::StatusCode, Json};
use eyre::WrapErr;
use maimai_http_client::is_maintenance_window_now;
use reqwest::Url;
use tracing::debug;

use maimai_parsers::parse_player_data_html;
use models::ParsedPlayerData;

use crate::error::Result;
use crate::state::AppState;

/// GET /api/player
/// Fetches and parses the player data from maimaidx-eng.com
pub async fn get_player(
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<ParsedPlayerData>)> {
    debug!("GET /api/player: fetching player data");

    if is_maintenance_window_now() {
        debug!("GET /api/player: maintenance window detected (04:00-07:00)");
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

    let url = Url::parse("https://maimaidx-eng.com/maimai-mobile/playerData/")
        .wrap_err("parse playerData url")
        .map_err(|e| crate::error::AppError::InternalError(e.to_string()))?;

    let bytes = client
        .get_bytes(&url)
        .await
        .wrap_err("fetch playerData url")
        .map_err(|e| crate::error::AppError::HttpClientError(e.to_string()))?;

    let html = String::from_utf8(bytes)
        .wrap_err("playerData response is not utf-8")
        .map_err(|e| crate::error::AppError::InternalError(e.to_string()))?;

    let player_data = parse_player_data_html(&html)
        .wrap_err("parse playerData html")
        .map_err(|e| crate::error::AppError::InternalError(e.to_string()))?;

    debug!(
        "GET /api/player: successfully fetched player data for {}",
        player_data.user_name
    );

    Ok((StatusCode::OK, Json(player_data)))
}
