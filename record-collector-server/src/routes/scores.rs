use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::{
    error::Result,
    routes::responses::{score_response_from_entry, ScoreResponse},
    state::AppState,
};
use models::ScoreEntry;

#[derive(Deserialize)]
pub(crate) struct SearchQuery {
    q: String,
}

pub(crate) async fn search_scores(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<Vec<ScoreResponse>>> {
    let search_term = format!("%{}%", params.q);

    let rows = sqlx::query_as::<_, ScoreEntry>(
        "SELECT title, chart_type, diff_category, achievement_x10000, rank, fc, sync, dx_score, dx_score_max
         FROM scores
         WHERE title LIKE ? AND achievement_x10000 IS NOT NULL
         ORDER BY title
         LIMIT 50"
    )
    .bind(&search_term)
    .fetch_all(&state.db_pool)
    .await?;

    let mut responses = Vec::with_capacity(rows.len());
    for entry in rows {
        responses.push(score_response_from_entry(entry)?);
    }

    Ok(Json(responses))
}

pub(crate) async fn get_score(
    State(state): State<AppState>,
    Path((title, chart_type, diff_category)): Path<(String, String, String)>,
) -> Result<Json<ScoreResponse>> {
    let score = sqlx::query_as::<_, ScoreEntry>(
        "SELECT title, chart_type, diff_category, achievement_x10000, rank, fc, sync, dx_score, dx_score_max
         FROM scores
         WHERE title = ? AND chart_type = ? AND diff_category = ? AND achievement_x10000 IS NOT NULL"
    )
    .bind(&title)
    .bind(&chart_type)
    .bind(&diff_category)
    .fetch_optional(&state.db_pool)
    .await?;

    if let Some(entry) = score {
        return Ok(Json(score_response_from_entry(entry)?));
    }

    Err(crate::error::AppError::NotFound(format!(
        "Score not found for title='{}', chart_type='{}', diff_category='{}'",
        title, chart_type, diff_category
    )))
}

pub(crate) async fn get_all_rated_scores(
    State(state): State<AppState>,
) -> Result<Json<Vec<ScoreResponse>>> {
    let rows = sqlx::query_as::<_, ScoreEntry>(
        "SELECT title, chart_type, diff_category, achievement_x10000, rank, fc, sync, dx_score, dx_score_max
         FROM scores
         WHERE achievement_x10000 IS NOT NULL
         ORDER BY title, chart_type, diff_category"
    )
    .fetch_all(&state.db_pool)
    .await?;

    let mut responses = Vec::with_capacity(rows.len());
    for entry in rows {
        responses.push(score_response_from_entry(entry)?);
    }

    Ok(Json(responses))
}
