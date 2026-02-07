use axum::{
    extract::{Path, Query, State},
    Json,
};
use models::{ChartType, DifficultyCategory, MaimaiVersion};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};
use strum::IntoEnumIterator;

use crate::error::{AppError, Result};
use crate::state::AppState;

#[derive(Serialize)]
pub(crate) struct SongSheetResponse {
    chart_type: ChartType,
    difficulty: DifficultyCategory,
    level: String,
    internal_level: Option<f32>,
    user_level: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct SongMetadataResponse {
    internal_level: Option<f32>,
    user_level: Option<String>,
    image_name: Option<String>,
    version: Option<String>,
    bucket: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct SongSelectionStatsResponse {
    level_song_count: usize,
    filtered_song_count: usize,
}

#[derive(Serialize)]
pub(crate) struct SongResponse {
    title: String,
    version: Option<String>,
    image_name: Option<String>,
    sheets: Vec<SongSheetResponse>,
    selection_stats: SongSelectionStatsResponse,
}

#[derive(Serialize)]
pub(crate) struct SongVersionResponse {
    version_index: u8,
    version_name: String,
    song_count: usize,
}

#[derive(Serialize)]
pub(crate) struct SongVersionsListResponse {
    versions: Vec<SongVersionResponse>,
}

pub(crate) async fn random_song_by_level(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<SongResponse>> {
    let min_level = parse_level_param(&params, "min_level")?;
    let max_level = parse_level_param(&params, "max_level")?;
    if min_level > max_level {
        return Err(AppError::JsonError(
            "min_level must be less than or equal to max_level".to_string(),
        ));
    }

    let include_chart_types = parse_chart_type_filter(&params)?;
    let include_difficulties = parse_difficulty_filter(&params)?;
    let include_versions = parse_version_filter(&params)?;

    let mut candidates = Vec::new();
    let mut level_song_count = 0usize;
    let song_data_root = state
        .song_data_root
        .read()
        .map_err(|_| AppError::IoError("Failed to read song data".to_string()))?;

    for song in song_data_root.iter() {
        let mut song_has_sheet_in_level_range = false;
        let song_version_enum = song.version.as_deref().and_then(MaimaiVersion::from_name);
        let song_passes_version_filter = include_versions.as_ref().is_none_or(|allowed| {
            song_version_enum.is_some_and(|version| allowed.contains(&version))
        });

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

            song_has_sheet_in_level_range = true;

            let Some(chart_type) = parse_sheet_chart_type(&sheet.sheet_type) else {
                continue;
            };
            let Some(difficulty) = parse_sheet_difficulty(&sheet.difficulty) else {
                continue;
            };

            if !song_passes_version_filter {
                continue;
            }

            if include_chart_types
                .as_ref()
                .is_some_and(|allowed| !allowed.contains(&chart_type))
            {
                continue;
            }

            if include_difficulties
                .as_ref()
                .is_some_and(|allowed| !allowed.contains(&difficulty))
            {
                continue;
            }

            sheets.push(SongSheetResponse {
                chart_type,
                difficulty,
                level: sheet.level.clone(),
                internal_level,
                user_level: sheet.user_level.clone(),
            });
        }

        if song_has_sheet_in_level_range {
            level_song_count += 1;
        }

        if !sheets.is_empty() {
            candidates.push(SongResponse {
                title: song.title.clone(),
                version: song.version.clone(),
                image_name: song.image_name.clone(),
                sheets,
                selection_stats: SongSelectionStatsResponse {
                    level_song_count: 0,
                    filtered_song_count: 0,
                },
            });
        }
    }

    let filtered_song_count = candidates.len();
    if filtered_song_count == 0 {
        return Err(AppError::NotFound(format!(
            "No songs found with internal_level between {} and {} after filters",
            min_level, max_level
        )));
    }

    let idx = select_random_index(filtered_song_count);
    let mut selected = candidates.swap_remove(idx);
    selected.selection_stats = SongSelectionStatsResponse {
        level_song_count,
        filtered_song_count,
    };

    Ok(Json(selected))
}

pub(crate) async fn list_versions(
    State(state): State<AppState>,
) -> Result<Json<SongVersionsListResponse>> {
    let song_data_root = state
        .song_data_root
        .read()
        .map_err(|_| AppError::IoError("Failed to read song data".to_string()))?;

    let mut counts: HashMap<MaimaiVersion, usize> = HashMap::new();
    for song in song_data_root.iter() {
        let Some(version_name) = song.version.as_deref() else {
            continue;
        };
        let Some(version) = MaimaiVersion::from_name(version_name) else {
            continue;
        };

        *counts.entry(version).or_insert(0) += 1;
    }

    let versions = MaimaiVersion::iter()
        .map(|version| SongVersionResponse {
            version_index: version.as_index(),
            version_name: version.as_str().to_string(),
            song_count: counts.get(&version).copied().unwrap_or(0),
        })
        .collect();

    Ok(Json(SongVersionsListResponse { versions }))
}

fn parse_level_param(params: &HashMap<String, String>, key: &str) -> Result<f32> {
    let value = params
        .get(key)
        .ok_or_else(|| AppError::JsonError(format!("missing query param: {}", key)))?;
    value
        .parse::<f32>()
        .map_err(|_| AppError::JsonError(format!("{} must be a valid number", key)))
}

fn parse_csv_param<'a>(params: &'a HashMap<String, String>, key: &str) -> Option<Vec<&'a str>> {
    let raw = params.get(key)?;
    if raw.trim().is_empty() {
        return Some(Vec::new());
    }

    let values = raw
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();

    Some(values)
}

fn parse_chart_type_filter(params: &HashMap<String, String>) -> Result<Option<HashSet<ChartType>>> {
    let Some(values) = parse_csv_param(params, "chart_types") else {
        return Ok(None);
    };

    let mut parsed = HashSet::new();
    for value in values {
        let Some(chart_type) = parse_chart_type_query_value(value) else {
            return Err(AppError::JsonError(format!(
                "invalid chart type: {} (expected STD or DX)",
                value
            )));
        };
        parsed.insert(chart_type);
    }

    Ok(Some(parsed))
}

fn parse_difficulty_filter(
    params: &HashMap<String, String>,
) -> Result<Option<HashSet<DifficultyCategory>>> {
    let Some(values) = parse_csv_param(params, "include_difficulties") else {
        return Ok(None);
    };

    let mut parsed = HashSet::new();
    for value in values {
        let index = value.parse::<u8>().map_err(|_| {
            AppError::JsonError(format!(
                "invalid include_difficulties value: {} (expected numeric difficulty index)",
                value
            ))
        })?;
        let Some(difficulty) = DifficultyCategory::from_index(index) else {
            return Err(AppError::JsonError(format!(
                "unknown difficulty index: {}",
                index
            )));
        };
        parsed.insert(difficulty);
    }

    Ok(Some(parsed))
}

fn parse_version_filter(
    params: &HashMap<String, String>,
) -> Result<Option<HashSet<MaimaiVersion>>> {
    let Some(values) = parse_csv_param(params, "include_versions") else {
        return Ok(None);
    };

    let mut parsed = HashSet::new();
    for value in values {
        let index = value.parse::<u8>().map_err(|_| {
            AppError::JsonError(format!(
                "invalid include_versions value: {} (expected numeric version_index)",
                value
            ))
        })?;

        let Some(version) = MaimaiVersion::from_index(index) else {
            return Err(AppError::JsonError(format!(
                "unknown version_index: {}",
                index
            )));
        };
        parsed.insert(version);
    }

    Ok(Some(parsed))
}

fn parse_sheet_chart_type(sheet_type: &str) -> Option<ChartType> {
    ChartType::from_lowercase(sheet_type)
}

fn parse_sheet_difficulty(difficulty: &str) -> Option<DifficultyCategory> {
    DifficultyCategory::from_lowercase(difficulty)
}

fn parse_chart_type_query_value(value: &str) -> Option<ChartType> {
    value.trim().parse::<ChartType>().ok()
}

fn select_random_index(len: usize) -> usize {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    (nanos % len as u128) as usize
}

pub(crate) async fn get_song_metadata(
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
    let song_data_root = state
        .song_data_root
        .read()
        .map_err(|_| AppError::IoError("Failed to read song data".to_string()))?;

    for song in song_data_root.iter() {
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
                        user_level: sheet.user_level.clone(),
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
