use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Serialize;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{AppError, Result};
use crate::state::AppState;

#[derive(Serialize)]
pub struct SongSheetResponse {
    chart_type: String,
    difficulty: String,
    level: String,
    internal_level: Option<f32>,
}

#[derive(Serialize)]
pub struct SongMetadataResponse {
    internal_level: Option<f32>,
    image_name: Option<String>,
    version: Option<String>,
    bucket: Option<String>,
}

#[derive(Serialize)]
pub struct SongResponse {
    title: String,
    version: Option<String>,
    image_name: Option<String>,
    sheets: Vec<SongSheetResponse>,
}

pub async fn random_song_by_level(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<SongResponse>> {
    let min_level = parse_level_param(&params, "min_level")?;
    let max_level = parse_level_param(&params, "max_level")?;

    let mut candidates = Vec::new();

    for song in state.song_data_root.iter() {
        let mut sheets = Vec::new();

        for sheet in &song.sheets {
            let internal_level = sheet
                .internal_level
                .as_deref()
                .and_then(|value| value.trim().parse::<f32>().ok());

            let Some(level) = internal_level else {
                continue;
            };

            if level < min_level || level > max_level {
                continue;
            }

            sheets.push(SongSheetResponse {
                chart_type: sheet.sheet_type.clone(),
                difficulty: sheet.difficulty.clone(),
                level: sheet.level.clone(),
                internal_level,
            });
        }

        if !sheets.is_empty() {
            candidates.push(SongResponse {
                title: song.title.clone(),
                version: song.version.clone(),
                image_name: song.image_name.clone(),
                sheets,
            });
        }
    }

    if candidates.is_empty() {
        return Err(AppError::NotFound(format!(
            "No songs found with internal_level between {} and {}",
            min_level, max_level
        )));
    }

    let idx = select_random_index(candidates.len());
    let selected = candidates.swap_remove(idx);

    Ok(Json(selected))
}

fn parse_level_param(params: &HashMap<String, String>, key: &str) -> Result<f32> {
    let value = params
        .get(key)
        .ok_or_else(|| AppError::JsonError(format!("missing query param: {}", key)))?;
    value
        .parse::<f32>()
        .map_err(|_| AppError::JsonError(format!("{} must be a valid number", key)))
}

fn select_random_index(len: usize) -> usize {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    (nanos % len as u128) as usize
}

pub async fn get_song_metadata(
    State(state): State<AppState>,
    Path((title, chart_type, diff_category)): Path<(String, String, String)>,
) -> Result<Json<SongMetadataResponse>> {
    // URL-decode path parameters
    let title = urlencoding::decode(&title)
        .map_err(|_| AppError::JsonError("Invalid title encoding".to_string()))?
        .into_owned();
    let chart_type = urlencoding::decode(&chart_type)
        .map_err(|_| AppError::JsonError("Invalid chart_type encoding".to_string()))?
        .into_owned();
    let diff_category = urlencoding::decode(&diff_category)
        .map_err(|_| AppError::JsonError("Invalid diff_category encoding".to_string()))?
        .into_owned();

    // Search for matching song in song_data_root
    for song in state.song_data_root.iter() {
        if song.title.eq_ignore_ascii_case(&title) {
            // Found matching song, now search for matching sheet
            for sheet in &song.sheets {
                if sheet.sheet_type.eq_ignore_ascii_case(&chart_type)
                    && sheet.difficulty.eq_ignore_ascii_case(&diff_category)
                {
                    // Found matching sheet
                    let internal_level = sheet
                        .internal_level
                        .as_deref()
                        .and_then(|value| value.trim().parse::<f32>().ok());

                    let bucket = song.version.as_ref().map(|v| {
                        if is_new_version(v) {
                            "New".to_string()
                        } else {
                            "Old".to_string()
                        }
                    });

                    return Ok(Json(SongMetadataResponse {
                        internal_level,
                        image_name: song.image_name.clone(),
                        version: song.version.clone(),
                        bucket,
                    }));
                }
            }
        }
    }

    // Not found
    Err(AppError::NotFound(format!(
        "Song not found: {} / {} / {}",
        title, chart_type, diff_category
    )))
}

fn is_new_version(version: &str) -> bool {
    matches!(version, "PRiSM PLUS" | "CiRCLE")
}
