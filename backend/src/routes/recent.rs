use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;

use crate::{error::Result, routes::responses::PlayRecordResponse, state::AppState};
use models::PlayRecord;

#[derive(Deserialize)]
pub struct RecentQuery {
    #[serde(default = "default_limit")]
    limit: i64,
}

fn default_limit() -> i64 {
    50
}

pub async fn get_recent(
    State(state): State<AppState>,
    Query(params): Query<RecentQuery>,
) -> Result<Json<Vec<PlayRecordResponse>>> {
    let limit = params.limit.clamp(1, 500);

    let rows = sqlx::query_as::<_, PlayRecord>(
        "SELECT played_at_unixtime, played_at, track, title, chart_type, diff_category, level, 
                achievement_x10000, score_rank, fc, sync, dx_score, dx_score_max, 
                credit_play_count, achievement_new_record, first_play
         FROM playlogs
         ORDER BY played_at_unixtime DESC
         LIMIT ?",
    )
    .bind(limit)
    .fetch_all(&state.db_pool)
    .await?;

    let responses = rows
        .into_iter()
        .map(|record| PlayRecordResponse::from_record(record, &state))
        .collect();

    Ok(Json(responses))
}
