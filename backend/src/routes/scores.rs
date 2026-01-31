use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use models::ScoreEntry;
use crate::{
    error::Result,
    routes::responses::ScoreResponse,
    state::AppState,
};

#[derive(Deserialize)]
pub struct SearchQuery {
    q: String,
}

pub async fn search_scores(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<Vec<ScoreResponse>>> {
    let search_term = format!("%{}%", params.q);

    let rows = sqlx::query_as::<_, ScoreEntry>(
        "SELECT title, chart_type, diff_category, level, achievement_x10000, rank, fc, sync, dx_score, dx_score_max, source_idx
         FROM scores
         WHERE title LIKE ? AND achievement_x10000 IS NOT NULL
         ORDER BY title
         LIMIT 50"
    )
    .bind(&search_term)
    .fetch_all(&state.db_pool)
    .await?;

    let responses = rows.into_iter()
        .map(|entry| ScoreResponse::from_entry(entry, &state))
        .collect();

    Ok(Json(responses))
}

pub async fn get_score(
    State(state): State<AppState>,
    Path((title, chart_type, diff_category)): Path<(String, String, String)>,
) -> Result<Json<ScoreResponse>> {
    let score = sqlx::query_as::<_, ScoreEntry>(
        "SELECT title, chart_type, diff_category, level, achievement_x10000, rank, fc, sync, dx_score, dx_score_max, source_idx
         FROM scores
         WHERE title = ? AND chart_type = ? AND diff_category = ? AND achievement_x10000 IS NOT NULL"
    )
    .bind(&title)
    .bind(&chart_type)
    .bind(&diff_category)
    .fetch_optional(&state.db_pool)
    .await?;

    score
        .map(|entry| Json(ScoreResponse::from_entry(entry, &state)))
        .ok_or_else(|| crate::error::AppError::NotFound(format!(
            "Score not found for title='{}', chart_type='{}', diff_category='{}'",
            title, chart_type, diff_category
        )))
}
