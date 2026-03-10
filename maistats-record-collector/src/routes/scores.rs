use axum::{
    Json,
    extract::{Query, State},
};
use serde::Deserialize;

use crate::{
    error::{AppError, Result},
    routes::responses::{ScoreApiResponse, score_response_from_entry},
    state::AppState,
};
use models::{SongDetailScoreApiResponse, StoredScoreEntry};

#[derive(Deserialize)]
pub(crate) struct SearchQuery {
    q: String,
}

#[derive(Deserialize)]
pub(crate) struct ScoreItemQuery {
    title: String,
    genre: String,
    artist: String,
    chart_type: String,
    diff_category: String,
}

#[derive(Deserialize)]
pub(crate) struct SongScoresQuery {
    title: String,
    genre: String,
    artist: String,
}

pub(crate) async fn search_scores(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<Vec<ScoreApiResponse>>> {
    let search_term = format!("%{}%", params.q);

    let rows = sqlx::query_as::<_, StoredScoreEntry>(
        "SELECT title, genre, artist, chart_type, diff_category, achievement_x10000, rank, fc, sync, dx_score, dx_score_max, last_played_at, play_count
         FROM scores
         WHERE title LIKE ? AND achievement_x10000 IS NOT NULL
         ORDER BY title, genre, artist, chart_type, diff_category
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
    Query(params): Query<ScoreItemQuery>,
) -> Result<Json<ScoreApiResponse>> {
    let score = sqlx::query_as::<_, StoredScoreEntry>(
        "SELECT title, genre, artist, chart_type, diff_category, achievement_x10000, rank, fc, sync, dx_score, dx_score_max, last_played_at, play_count
         FROM scores
         WHERE title = ? AND genre = ? AND artist = ? AND chart_type = ? AND diff_category = ? AND achievement_x10000 IS NOT NULL"
    )
    .bind(&params.title)
    .bind(&params.genre)
    .bind(&params.artist)
    .bind(&params.chart_type)
    .bind(&params.diff_category)
    .fetch_optional(&state.db_pool)
    .await?;

    let Some(entry) = score else {
        return Err(AppError::NotFound(format!(
            "Score not found for title='{}', genre='{}', artist='{}', chart_type='{}', diff_category='{}'",
            params.title, params.genre, params.artist, params.chart_type, params.diff_category
        )));
    };

    Ok(Json(score_response_from_entry(entry)?))
}

pub(crate) async fn get_all_rated_scores(
    State(state): State<AppState>,
) -> Result<Json<Vec<ScoreApiResponse>>> {
    let rows = sqlx::query_as::<_, StoredScoreEntry>(
        "SELECT title, genre, artist, chart_type, diff_category, achievement_x10000, rank, fc, sync, dx_score, dx_score_max, last_played_at, play_count
         FROM scores
         WHERE achievement_x10000 IS NOT NULL
         ORDER BY title, genre, artist, chart_type, diff_category"
    )
    .fetch_all(&state.db_pool)
    .await?;

    let mut responses = Vec::with_capacity(rows.len());
    for entry in rows {
        responses.push(score_response_from_entry(entry)?);
    }

    Ok(Json(responses))
}

pub(crate) async fn get_song_detail_scores(
    State(state): State<AppState>,
    Query(params): Query<SongScoresQuery>,
) -> Result<Json<Vec<SongDetailScoreApiResponse>>> {
    let rows = sqlx::query_as::<_, StoredScoreEntry>(
        "SELECT title, genre, artist, chart_type, diff_category, achievement_x10000, rank, fc, sync, dx_score, dx_score_max, last_played_at, play_count
         FROM scores
         WHERE title = ? AND genre = ? AND artist = ? AND achievement_x10000 IS NOT NULL
         ORDER BY chart_type, diff_category"
    )
    .bind(&params.title)
    .bind(&params.genre)
    .bind(&params.artist)
    .fetch_all(&state.db_pool)
    .await?;

    if rows.is_empty() {
        return Err(AppError::NotFound(format!(
            "No played scores found for title='{}', genre='{}', artist='{}'",
            params.title, params.genre, params.artist
        )));
    }

    let mut responses = Vec::with_capacity(rows.len());
    for entry in rows {
        let score = score_response_from_entry(entry)?;
        responses.push(SongDetailScoreApiResponse {
            title: score.title,
            genre: score.genre,
            artist: score.artist,
            chart_type: score.chart_type,
            diff_category: score.diff_category,
            achievement_x10000: score.achievement_x10000,
            rank: score.rank,
            fc: score.fc,
            sync: score.sync,
            dx_score: score.dx_score,
            dx_score_max: score.dx_score_max,
            last_played_at: score.last_played_at,
            play_count: score.play_count,
        });
    }

    Ok(Json(responses))
}
