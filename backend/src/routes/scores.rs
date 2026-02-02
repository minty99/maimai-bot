use axum::{
    extract::{Path, Query, State},
    Json,
};
use rand::seq::SliceRandom;
use serde::Deserialize;

use crate::{error::Result, routes::responses::ScoreResponse, state::AppState};
use models::ScoreEntry;

#[derive(Deserialize)]
pub struct SearchQuery {
    q: String,
}

#[derive(Deserialize)]
pub struct RandomSongQuery {
    min_level: f32,
    max_level: f32,
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

    let responses = rows
        .into_iter()
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
        .ok_or_else(|| {
            crate::error::AppError::NotFound(format!(
                "Score not found for title='{}', chart_type='{}', diff_category='{}'",
                title, chart_type, diff_category
            ))
        })
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

    let responses = rows
        .into_iter()
        .map(|entry| ScoreResponse::from_entry(entry, &state))
        .collect();

    Ok(Json(responses))
}

pub async fn random_song_by_level(
    State(state): State<AppState>,
    Query(params): Query<RandomSongQuery>,
) -> Result<Json<ScoreResponse>> {
    let rows = sqlx::query_as::<_, ScoreEntry>(
        "SELECT title, chart_type, diff_category, level, achievement_x10000, rank, fc, sync, dx_score, dx_score_max, source_idx
         FROM scores
         WHERE achievement_x10000 IS NOT NULL
         ORDER BY title, chart_type, diff_category"
    )
    .fetch_all(&state.db_pool)
    .await?;

    let song_data = state.song_data.read().unwrap();

    let filtered: Vec<ScoreResponse> = rows
        .into_iter()
        .filter_map(|entry| {
            let internal_level =
                song_data.internal_level(&entry.title, &entry.chart_type, &entry.diff_category);
            let effective_internal =
                internal_level.or_else(|| crate::rating::fallback_internal_level(&entry.level));

            if let Some(level) = effective_internal {
                if level >= params.min_level && level <= params.max_level {
                    return Some(ScoreResponse::from_entry(entry, &state));
                }
            }
            None
        })
        .collect();

    drop(song_data);

    if filtered.is_empty() {
        return Err(crate::error::AppError::NotFound(format!(
            "No songs found with internal_level between {} and {}",
            params.min_level, params.max_level
        )));
    }

    let mut rng = rand::thread_rng();
    let random_song = filtered
        .choose(&mut rng)
        .ok_or_else(|| {
            crate::error::AppError::NotFound("Failed to select random song".to_string())
        })?
        .clone();

    Ok(Json(random_song))
}
