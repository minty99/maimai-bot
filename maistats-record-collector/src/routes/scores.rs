use axum::{
    Json,
    extract::{Query, State},
};
use serde::{Deserialize, Serialize};

use crate::{
    error::{AppError, Result, app_error_from_maimai},
    routes::responses::{ScoreApiResponse, score_response_from_entry},
    state::AppState,
    tasks::utils::{
        auth::ensure_session,
        scores::{
            RefreshSongScoresOutcome, RefreshSongScoresTarget,
            refresh_song_scores as refresh_song_scores_task,
        },
    },
};
use models::{SongDetailScoreApiResponse, StoredScoreEntry};

#[derive(Deserialize)]
pub(crate) struct SongScoresQuery {
    title: String,
    genre: String,
    artist: String,
}

#[derive(Deserialize)]
pub(crate) struct RefreshSongScoresRequest {
    title: String,
    genre: String,
    artist: String,
}

#[derive(Serialize)]
pub(crate) struct RefreshSongScoresResponse {
    detail_pages_refreshed: usize,
    rows_written: usize,
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

pub(crate) async fn refresh_song_scores(
    State(state): State<AppState>,
    Json(payload): Json<RefreshSongScoresRequest>,
) -> Result<Json<RefreshSongScoresResponse>> {
    let target = RefreshSongScoresTarget {
        title: payload.title.trim().to_string(),
        genre: payload.genre.trim().to_string(),
        artist: payload.artist.trim().to_string(),
    };

    let mut client = state.maimai_client().map_err(app_error_from_maimai)?;
    ensure_session(&mut client)
        .await
        .map_err(app_error_from_maimai)?;

    let outcome: RefreshSongScoresOutcome =
        refresh_song_scores_task(&state.db_pool, &mut client, &target)
            .await
            .map_err(app_error_from_maimai)?;

    Ok(Json(RefreshSongScoresResponse {
        detail_pages_refreshed: outcome.detail_pages_refreshed,
        rows_written: outcome.rows_written,
    }))
}
