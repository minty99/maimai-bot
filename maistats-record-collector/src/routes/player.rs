use axum::{Json, extract::State, http::StatusCode};
use tracing::debug;

use models::ParsedPlayerProfile;

use crate::error::{AppError, Result};
use crate::state::AppState;
use crate::tasks::utils::player::load_stored_player_profile;

/// GET /api/player
/// Returns the latest stored player profile snapshot from SQLite.
pub(crate) async fn get_player(
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<ParsedPlayerProfile>)> {
    debug!("GET /api/player: loading stored player profile");

    let Some(player_profile) = load_stored_player_profile(&state.db_pool)
        .await
        .map_err(AppError::from)?
    else {
        return Err(AppError::NotFound(
            "No stored player profile is available yet".to_string(),
        ));
    };

    Ok((StatusCode::OK, Json(player_profile)))
}
