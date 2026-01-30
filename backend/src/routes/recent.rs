use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;

use models::PlayRecord;
use crate::{error::Result, state::AppState};

#[derive(Deserialize)]
pub struct RecentQuery {
    #[serde(default = "default_limit")]
    limit: i64,
}

fn default_limit() -> i64 {
    50
}

/// GET /api/recent?limit=N
/// Query DB: SELECT * FROM playlogs ORDER BY played_at_unixtime DESC LIMIT ?
pub async fn get_recent(
    State(state): State<AppState>,
    Query(params): Query<RecentQuery>,
) -> Result<Json<Vec<PlayRecord>>> {
    let limit = params.limit.max(1).min(500); // Clamp between 1 and 500

    let rows = sqlx::query_as::<_, PlayRecord>(
        "SELECT played_at_unixtime, played_at, track, title, chart_type, diff_category, level, 
                achievement_x10000, score_rank, fc, sync, dx_score, dx_score_max, 
                credit_play_count, achievement_new_record, first_play
         FROM playlogs
         ORDER BY played_at_unixtime DESC
         LIMIT ?"
    )
    .bind(limit)
    .fetch_all(&state.db_pool)
    .await?;

    Ok(Json(rows))
}
