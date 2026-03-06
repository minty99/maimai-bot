use axum::{
    Json,
    extract::{Path, Query, State},
};
use models::{ChartType, DifficultyCategory, MaimaiVersion, SongChartRegion};
use serde::Deserialize;
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
    version: Option<String>,
    internal_level: Option<f32>,
    user_level: Option<String>,
    region: SongChartRegion,
}

#[derive(Serialize)]
pub(crate) struct SongMetadataResponse {
    level: Option<String>,
    internal_level: Option<f32>,
    user_level: Option<String>,
    image_name: Option<String>,
    version: Option<String>,
    genre: String,
    artist: String,
    region: SongChartRegion,
}

#[derive(Serialize)]
pub(crate) struct SongInfoResponse {
    title: String,
    genre: String,
    artist: String,
    image_name: Option<String>,
    sheets: Vec<SongSheetResponse>,
}

#[derive(Serialize)]
pub(crate) struct SongCatalogResponse {
    songs: Vec<SongInfoResponse>,
}

#[derive(Serialize)]
pub(crate) struct SongSelectionStatsResponse {
    level_song_count: usize,
    filtered_song_count: usize,
}

#[derive(Serialize)]
pub(crate) struct SongResponse {
    title: String,
    genre: String,
    artist: String,
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

#[derive(Debug, Deserialize)]
pub(crate) struct SongMetadataQuery {
    title: String,
    genre: String,
    artist: String,
    chart_type: String,
    diff_category: String,
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
        let mut sheets = Vec::new();

        for sheet in &song.sheets {
            if !is_intl_sheet(sheet) {
                continue;
            }

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

            let Some(chart_type) = parse_sheet_chart_type(&sheet.chart_type) else {
                continue;
            };
            let Some(difficulty) = parse_sheet_difficulty(&sheet.difficulty) else {
                continue;
            };

            let sheet_version = sheet
                .version_name
                .clone()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty());
            let sheet_version_enum = sheet_version.as_deref().and_then(MaimaiVersion::from_name);

            if include_versions.as_ref().is_some_and(|allowed| {
                sheet_version_enum.is_none_or(|version| !allowed.contains(&version))
            }) {
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
                version: sheet_version,
                internal_level,
                user_level: sheet.user_level.clone(),
                region: sheet.region.clone(),
            });
        }

        if song_has_sheet_in_level_range {
            level_song_count += 1;
        }

        if !sheets.is_empty() {
            candidates.push(SongResponse {
                title: song.title.clone(),
                genre: song.genre.clone(),
                artist: song.artist.clone(),
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

    let mut version_song_titles: HashMap<MaimaiVersion, HashSet<String>> = HashMap::new();
    for song in song_data_root.iter() {
        let mut seen_versions_for_song = HashSet::new();
        for sheet in &song.sheets {
            let Some(version) = parse_intl_sheet_version(sheet) else {
                continue;
            };
            seen_versions_for_song.insert(version);
        }

        for version in seen_versions_for_song {
            version_song_titles
                .entry(version)
                .or_default()
                .insert(song.title.clone());
        }
    }

    let versions = MaimaiVersion::iter()
        .map(|version| SongVersionResponse {
            version_index: version.as_index(),
            version_name: version.as_str().to_string(),
            song_count: version_song_titles.get(&version).map_or(0, HashSet::len),
        })
        .collect();

    Ok(Json(SongVersionsListResponse { versions }))
}

fn build_song_sheet_response(sheet: &models::SongCatalogChart) -> Result<SongSheetResponse> {
    let chart_type = parse_sheet_chart_type(&sheet.chart_type).ok_or_else(|| {
        AppError::JsonError(format!(
            "unknown chart type in song data: {}",
            sheet.chart_type
        ))
    })?;
    let difficulty = parse_sheet_difficulty(&sheet.difficulty).ok_or_else(|| {
        AppError::JsonError(format!(
            "unknown difficulty in song data: {}",
            sheet.difficulty
        ))
    })?;

    Ok(SongSheetResponse {
        chart_type,
        difficulty,
        level: sheet.level.clone(),
        version: sheet
            .version_name
            .clone()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty()),
        internal_level: sheet
            .internal_level
            .as_deref()
            .and_then(|value| value.trim().parse::<f32>().ok()),
        user_level: sheet.user_level.clone(),
        region: sheet.region.clone(),
    })
}

fn build_song_info_response(song: &models::SongCatalogSong) -> Result<SongInfoResponse> {
    let sheets = song
        .sheets
        .iter()
        .map(build_song_sheet_response)
        .collect::<Result<Vec<_>>>()?;

    Ok(SongInfoResponse {
        title: song.title.clone(),
        genre: song.genre.clone(),
        artist: song.artist.clone(),
        image_name: song.image_name.clone(),
        sheets,
    })
}

fn song_matches_identity(
    song: &models::SongCatalogSong,
    title: &str,
    genre: &str,
    artist: &str,
) -> bool {
    song.title == title && song.genre == genre && song.artist == artist
}

pub(crate) async fn list_song_info(
    State(state): State<AppState>,
) -> Result<Json<SongCatalogResponse>> {
    let song_data_root = state
        .song_data_root
        .read()
        .map_err(|_| AppError::IoError("Failed to read song data".to_string()))?;

    let songs = song_data_root
        .iter()
        .map(build_song_info_response)
        .collect::<Result<Vec<_>>>()?;

    Ok(Json(SongCatalogResponse { songs }))
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

fn is_intl_sheet(sheet: &models::SongCatalogChart) -> bool {
    sheet.region.intl
}

fn parse_intl_sheet_version(sheet: &models::SongCatalogChart) -> Option<MaimaiVersion> {
    if !is_intl_sheet(sheet) {
        return None;
    }
    let version_name = sheet.version_name.as_deref()?;
    MaimaiVersion::from_name(version_name)
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
                if sheet.chart_type.eq_ignore_ascii_case(&chart_type)
                    && sheet.difficulty.eq_ignore_ascii_case(&diff_category)
                {
                    // Found matching sheet
                    let internal_level = sheet
                        .internal_level
                        .as_deref()
                        .and_then(|value| value.trim().parse::<f32>().ok());
                    let version = sheet
                        .version_name
                        .clone()
                        .map(|v| v.trim().to_string())
                        .filter(|v| !v.is_empty());

                    return Ok(Json(SongMetadataResponse {
                        level: Some(sheet.level.clone()),
                        internal_level,
                        user_level: sheet.user_level.clone(),
                        image_name: song.image_name.clone(),
                        version,
                        genre: song.genre.clone(),
                        artist: song.artist.clone(),
                        region: sheet.region.clone(),
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

pub(crate) async fn get_song_metadata_item(
    State(state): State<AppState>,
    Query(params): Query<SongMetadataQuery>,
) -> Result<Json<SongMetadataResponse>> {
    let song_data_root = state
        .song_data_root
        .read()
        .map_err(|_| AppError::IoError("Failed to read song data".to_string()))?;

    for song in song_data_root.iter() {
        if !song_matches_identity(song, &params.title, &params.genre, &params.artist) {
            continue;
        }

        for sheet in &song.sheets {
            if sheet.chart_type.eq_ignore_ascii_case(&params.chart_type)
                && sheet.difficulty.eq_ignore_ascii_case(&params.diff_category)
            {
                let internal_level = sheet
                    .internal_level
                    .as_deref()
                    .and_then(|value| value.trim().parse::<f32>().ok());
                let version = sheet
                    .version_name
                    .clone()
                    .map(|v| v.trim().to_string())
                    .filter(|v| !v.is_empty());

                return Ok(Json(SongMetadataResponse {
                    level: Some(sheet.level.clone()),
                    internal_level,
                    user_level: sheet.user_level.clone(),
                    image_name: song.image_name.clone(),
                    version,
                    genre: song.genre.clone(),
                    artist: song.artist.clone(),
                    region: sheet.region.clone(),
                }));
            }
        }
    }

    Err(AppError::NotFound(format!(
        "Song metadata not found: {} / {} / {} / {} / {}",
        params.title, params.genre, params.artist, params.chart_type, params.diff_category
    )))
}

pub(crate) async fn get_song_info_by_title(
    State(state): State<AppState>,
    Path(title): Path<String>,
) -> Result<Json<SongInfoResponse>> {
    let title = urlencoding::decode(&title)
        .map_err(|_| AppError::JsonError("Invalid title encoding".to_string()))?
        .into_owned();

    let song_data_root = state
        .song_data_root
        .read()
        .map_err(|_| AppError::IoError("Failed to read song data".to_string()))?;

    let Some(song) = song_data_root
        .iter()
        .find(|song| song.title.eq_ignore_ascii_case(&title))
    else {
        return Err(AppError::NotFound(format!("Song not found: {}", title)));
    };

    Ok(Json(build_song_info_response(song)?))
}

#[cfg(test)]
mod tests {
    use super::{build_song_info_response, is_intl_sheet, parse_intl_sheet_version};
    use models::{MaimaiVersion, SongCatalogChart, SongCatalogSong, SongChartRegion};

    #[test]
    fn intl_sheet_predicate_uses_region_flag() {
        let intl_sheet = SongCatalogChart {
            chart_type: "std".to_string(),
            difficulty: "basic".to_string(),
            level: "10".to_string(),
            version_name: Some("Splash".to_string()),
            internal_level: None,
            user_level: None,
            region: SongChartRegion {
                jp: false,
                intl: true,
            },
        };
        let jp_only_sheet = SongCatalogChart {
            chart_type: "std".to_string(),
            difficulty: "basic".to_string(),
            level: "10".to_string(),
            version_name: Some("Splash".to_string()),
            internal_level: None,
            user_level: None,
            region: SongChartRegion {
                jp: true,
                intl: false,
            },
        };

        assert!(is_intl_sheet(&intl_sheet));
        assert!(!is_intl_sheet(&jp_only_sheet));
    }

    #[test]
    fn parse_intl_sheet_version_skips_non_intl_sheet() {
        let jp_only_sheet = SongCatalogChart {
            chart_type: "std".to_string(),
            difficulty: "basic".to_string(),
            level: "10".to_string(),
            version_name: Some("Splash".to_string()),
            internal_level: None,
            user_level: None,
            region: SongChartRegion {
                jp: true,
                intl: false,
            },
        };
        let intl_sheet = SongCatalogChart {
            chart_type: "std".to_string(),
            difficulty: "basic".to_string(),
            level: "10".to_string(),
            version_name: Some("Splash".to_string()),
            internal_level: None,
            user_level: None,
            region: SongChartRegion {
                jp: true,
                intl: true,
            },
        };

        assert_eq!(parse_intl_sheet_version(&jp_only_sheet), None);
        assert_eq!(
            parse_intl_sheet_version(&intl_sheet),
            Some(MaimaiVersion::Splash)
        );
    }

    #[test]
    fn build_song_info_response_normalizes_sheet_fields() {
        let song = SongCatalogSong {
            title: "Test Song".to_string(),
            genre: "maimai".to_string(),
            artist: "".to_string(),
            image_name: Some("cover.png".to_string()),
            sheets: vec![SongCatalogChart {
                chart_type: "dx".to_string(),
                difficulty: "master".to_string(),
                level: "14+".to_string(),
                version_name: Some("  Buddies Plus  ".to_string()),
                internal_level: Some("14.7".to_string()),
                user_level: Some("14+".to_string()),
                region: SongChartRegion {
                    jp: true,
                    intl: false,
                },
            }],
        };

        let response = build_song_info_response(&song).expect("song info should build");

        assert_eq!(response.title, "Test Song");
        assert_eq!(response.genre, "maimai");
        assert_eq!(response.artist, "");
        assert_eq!(response.image_name.as_deref(), Some("cover.png"));
        assert_eq!(response.sheets.len(), 1);

        let sheet = &response.sheets[0];
        assert_eq!(sheet.chart_type, models::ChartType::Dx);
        assert_eq!(sheet.difficulty, models::DifficultyCategory::Master);
        assert_eq!(sheet.version.as_deref(), Some("Buddies Plus"));
        assert_eq!(sheet.internal_level, Some(14.7));
        assert!(sheet.region.jp);
        assert!(!sheet.region.intl);
    }
}
