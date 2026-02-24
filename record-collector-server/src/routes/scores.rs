use axum::{
    extract::{Path, Query, State},
    Json,
};
use eyre::WrapErr;
use maimai_parsers::{parse_scores_html, parse_song_detail_html};
use reqwest::Url;
use serde::Deserialize;
use std::collections::HashSet;

use crate::{
    error::{AppError, Result},
    http_client::{is_maintenance_window_now, MaimaiClient},
    routes::responses::{score_response_from_entry, ScoreResponse},
    state::AppState,
};
use models::{ScoreEntry, SongDetailScoreResponse};

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

pub(crate) async fn get_song_detail_scores(
    State(state): State<AppState>,
    Path(title): Path<String>,
) -> Result<Json<Vec<SongDetailScoreResponse>>> {
    if is_maintenance_window_now() {
        return Err(AppError::Maintenance(
            "maimai DX NET maintenance window (04:00-07:00 local time)".to_string(),
        ));
    }

    let mut client = state
        .maimai_client()
        .wrap_err("create HTTP client")
        .map_err(|e| AppError::InternalError(e.to_string()))?;
    client
        .ensure_logged_in()
        .await
        .map_err(map_maintenance_or_http_client_error)?;

    let detail_indices = find_song_detail_indices_by_title(&client, &title)
        .await
        .map_err(map_maintenance_or_http_client_error)?;
    if detail_indices.is_empty() {
        return Err(AppError::NotFound(format!(
            "No song detail index found for title='{}'",
            title
        )));
    }

    let mut responses = Vec::new();
    for detail_idx in detail_indices {
        let url = Url::parse_with_params(
            "https://maimaidx-eng.com/maimai-mobile/record/musicDetail/",
            &[("idx", detail_idx.as_str())],
        )
        .wrap_err("parse musicDetail url")
        .map_err(|e| AppError::InternalError(e.to_string()))?;

        let bytes = client
            .get_bytes(&url)
            .await
            .map_err(map_maintenance_or_http_client_error)?;
        let html = String::from_utf8(bytes)
            .wrap_err("musicDetail response is not utf-8")
            .map_err(|e| AppError::InternalError(e.to_string()))?;
        let parsed = parse_song_detail_html(&html)
            .wrap_err("parse musicDetail html")
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        for difficulty in parsed.difficulties {
            let achievement_x10000 = difficulty
                .achievement_percent
                .map(|v| (v as f64 * 10000.0).round() as i64);
            if achievement_x10000.is_none() {
                continue;
            }
            responses.push(SongDetailScoreResponse {
                title: parsed.title.clone(),
                chart_type: difficulty.chart_type,
                diff_category: difficulty.diff_category,
                achievement_x10000,
                rank: difficulty.rank,
                fc: difficulty.fc,
                sync: difficulty.sync,
                dx_score: difficulty.dx_score,
                dx_score_max: difficulty.dx_score_max,
                last_played_at: difficulty.last_played_at,
                play_count: difficulty.play_count,
            });
        }
    }

    responses.sort_by_key(|score| (score.chart_type, score.diff_category));

    if responses.is_empty() {
        return Err(AppError::NotFound(format!(
            "No played song details found for title='{}'",
            title
        )));
    }

    Ok(Json(responses))
}

async fn find_song_detail_indices_by_title(
    client: &MaimaiClient,
    title: &str,
) -> eyre::Result<Vec<String>> {
    let target_norm = normalize_title_for_match(title);
    let mut seen = HashSet::new();
    let mut indices = Vec::new();

    for diff in 0u8..=4 {
        let url = scores_url(diff).wrap_err("build scores url")?;
        let bytes = client.get_bytes(&url).await.wrap_err("fetch scores url")?;
        let html = String::from_utf8(bytes).wrap_err("scores response is not utf-8")?;
        let entries = parse_scores_html(&html, diff).wrap_err("parse scores html")?;

        for entry in entries {
            if normalize_title_for_match(&entry.title) != target_norm {
                continue;
            }
            if let Some(idx) = entry.source_idx {
                let idx = idx.trim();
                if !idx.is_empty() && seen.insert(idx.to_string()) {
                    indices.push(idx.to_string());
                }
            }
        }
    }

    Ok(indices)
}

fn normalize_title_for_match(s: &str) -> String {
    s.trim()
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect()
}

fn map_maintenance_or_http_client_error(e: eyre::Error) -> AppError {
    let msg = e.to_string();
    let lowered = msg.to_ascii_lowercase();
    if lowered.contains("maintenance") {
        return AppError::Maintenance(msg);
    }
    AppError::HttpClientError(msg)
}

fn scores_url(diff: u8) -> eyre::Result<Url> {
    if diff > 4 {
        return Err(eyre::eyre!("diff must be 0..4"));
    }
    Url::parse(&format!(
        "https://maimaidx-eng.com/maimai-mobile/record/musicGenre/search/?genre=99&diff={diff}"
    ))
    .wrap_err("parse scores url")
}
