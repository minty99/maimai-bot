use axum::{
    Json,
    extract::{Query, State},
};
use serde::Deserialize;

use crate::{
    error::Result,
    routes::responses::{PlayRecordApiResponse, play_record_response_from_record},
    state::AppState,
};
use models::StoredPlayRecord;

#[derive(Deserialize)]
pub(crate) struct RecentQuery {
    #[serde(default = "default_limit")]
    limit: i64,
}

fn default_limit() -> i64 {
    50
}

fn requested_limit(limit: i64) -> i64 {
    limit.max(1)
}

pub(crate) async fn get_recent(
    State(state): State<AppState>,
    Query(params): Query<RecentQuery>,
) -> Result<Json<Vec<PlayRecordApiResponse>>> {
    let limit = requested_limit(params.limit);

    let rows = sqlx::query_as::<_, StoredPlayRecord>(
        "SELECT played_at_unixtime, played_at, track, title, genre, artist, chart_type, diff_category, 
                achievement_x10000, score_rank, fc, sync, dx_score, dx_score_max, 
                credit_id, achievement_new_record
         FROM playlogs
         ORDER BY played_at_unixtime DESC
         LIMIT ?",
    )
    .bind(limit)
    .fetch_all(&state.db_pool)
    .await?;

    let mut responses = Vec::with_capacity(rows.len());
    for record in rows {
        responses.push(play_record_response_from_record(record)?);
    }

    Ok(Json(responses))
}

#[cfg(test)]
mod tests {
    use super::requested_limit;

    #[test]
    fn requested_limit_keeps_large_values() {
        assert_eq!(requested_limit(501), 501);
        assert_eq!(requested_limit(5_000), 5_000);
    }

    #[test]
    fn requested_limit_clamps_non_positive_values_to_one() {
        assert_eq!(requested_limit(0), 1);
        assert_eq!(requested_limit(-10), 1);
    }
}
