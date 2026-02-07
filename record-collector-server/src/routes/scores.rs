use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::{
    error::Result,
    routes::responses::{score_response_from_entry, ScoreResponse},
    song_info_client::SongInfoClient,
    state::AppState,
};
use models::ScoreEntry;

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

    let song_info_client = SongInfoClient::new(
        state.config.song_info_server_url.clone(),
        state.http_client.clone(),
    );

    let mut responses = Vec::with_capacity(rows.len());
    for entry in rows {
        responses.push(score_response_from_entry(entry, &song_info_client).await?);
    }

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

    if let Some(entry) = score {
        let song_info_client = SongInfoClient::new(
            state.config.song_info_server_url.clone(),
            state.http_client.clone(),
        );
        return Ok(Json(
            score_response_from_entry(entry, &song_info_client).await?,
        ));
    }

    Err(crate::error::AppError::NotFound(format!(
        "Score not found for title='{}', chart_type='{}', diff_category='{}'",
        title, chart_type, diff_category
    )))
}

pub async fn get_all_rated_scores(
    State(state): State<AppState>,
) -> Result<Json<Vec<ScoreResponse>>> {
    let rows = sqlx::query_as::<_, ScoreEntry>(
        "SELECT title, chart_type, diff_category, level, achievement_x10000, rank, fc, sync, dx_score, dx_score_max, source_idx
         FROM scores
         WHERE achievement_x10000 IS NOT NULL
         ORDER BY title, chart_type, diff_category"
    )
    .fetch_all(&state.db_pool)
    .await?;

    let song_info_client = SongInfoClient::new(
        state.config.song_info_server_url.clone(),
        state.http_client.clone(),
    );

    let mut responses = Vec::with_capacity(rows.len());
    for entry in rows {
        responses.push(score_response_from_entry(entry, &song_info_client).await?);
    }

    Ok(Json(responses))
}
