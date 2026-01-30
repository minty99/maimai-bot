use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use models::ScoreEntry;
use crate::{error::Result, state::AppState};

#[derive(Deserialize)]
pub struct SearchQuery {
    q: String,
}

/// GET /api/scores/search?q=<title>
/// Query DB with LIKE %title%, filter achievement_x10000 IS NOT NULL, limit 50
pub async fn search_scores(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<Vec<ScoreEntry>>> {
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

    Ok(Json(rows))
}

/// GET /api/scores/:title/:chart_type/:diff_category
/// Query DB by primary key (title, chart_type, diff_category), filter achievement_x10000 IS NOT NULL
pub async fn get_score(
    State(state): State<AppState>,
    Path((title, chart_type, diff_category)): Path<(String, String, String)>,
) -> Result<Json<ScoreEntry>> {
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
        .map(Json)
        .ok_or_else(|| crate::error::AppError::NotFound(format!(
            "Score not found for title='{}', chart_type='{}', diff_category='{}'",
            title, chart_type, diff_category
        )))
}
