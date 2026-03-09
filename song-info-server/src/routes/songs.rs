use axum::{
    Json,
    extract::{Query, State},
};
use models::{
    ChartType, DifficultyCategory, MaimaiVersion, SongAliases, SongChartRegion, SongGenre,
};
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
    region: SongChartRegion,
}

#[derive(Serialize)]
pub(crate) struct SongMetadataResponse {
    title: String,
    chart_type: ChartType,
    diff_category: DifficultyCategory,
    level: Option<String>,
    internal_level: Option<f32>,
    image_name: Option<String>,
    version: Option<String>,
    genre: String,
    artist: String,
    aliases: SongAliases,
    region: SongChartRegion,
}

#[derive(Serialize)]
pub(crate) struct SongInfoResponse {
    title: String,
    genre: String,
    artist: String,
    image_name: Option<String>,
    aliases: SongAliases,
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
pub(crate) struct SongMetadataSearchRequest {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    genre: Option<String>,
    #[serde(default)]
    artist: Option<String>,
    #[serde(default)]
    chart_type: Option<String>,
    #[serde(default)]
    diff_category: Option<String>,
    #[serde(default)]
    limits: Option<usize>,
}

#[derive(Serialize)]
pub(crate) struct SongMetadataSearchResponse {
    total: usize,
    items: Vec<SongMetadataResponse>,
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
            let sheet_version_enum = sheet_version
                .as_deref()
                .and_then(|value| value.parse::<MaimaiVersion>().ok());

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
                region: sheet.region.clone(),
            });
        }

        if song_has_sheet_in_level_range {
            level_song_count += 1;
        }

        if !sheets.is_empty() {
            candidates.push(SongResponse {
                title: song.title.clone(),
                genre: song.genre.to_string(),
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
        genre: song.genre.to_string(),
        artist: song.artist.clone(),
        image_name: song.image_name.clone(),
        aliases: song.aliases.clone(),
        sheets,
    })
}

fn song_matches_search_request(
    song: &models::SongCatalogSong,
    title: Option<&str>,
    genre: Option<&SongGenre>,
    artist: Option<&str>,
) -> bool {
    title.is_none_or(|title| song.title == title)
        && genre.is_none_or(|genre| song.genre == *genre)
        && artist.is_none_or(|artist| song.artist == artist)
}

fn normalize_lookup_value(value: &str) -> String {
    value.trim().to_lowercase()
}

fn song_matches_flexible_search_request(
    song: &models::SongCatalogSong,
    title: Option<&str>,
    genre: Option<&SongGenre>,
    artist: Option<&str>,
) -> bool {
    title.is_none_or(|title| {
        let normalized_title = normalize_lookup_value(title);
        normalize_lookup_value(&song.title) == normalized_title
            || song
                .aliases
                .en
                .iter()
                .chain(song.aliases.ko.iter())
                .any(|alias| normalize_lookup_value(alias) == normalized_title)
    }) && genre.is_none_or(|genre| song.genre == *genre)
        && artist.is_none_or(|artist| {
            normalize_lookup_value(&song.artist) == normalize_lookup_value(artist)
        })
}

fn collect_song_metadata_items(
    songs: &[models::SongCatalogSong],
    params: &SongMetadataSearchRequest,
    parsed_genre: Option<&SongGenre>,
    parsed_chart_type: Option<ChartType>,
    parsed_diff_category: Option<DifficultyCategory>,
    use_flexible_match: bool,
) -> Vec<SongMetadataResponse> {
    let mut items = Vec::new();

    for song in songs {
        let matches = if use_flexible_match {
            song_matches_flexible_search_request(
                song,
                params.title.as_deref(),
                parsed_genre,
                params.artist.as_deref(),
            )
        } else {
            song_matches_search_request(
                song,
                params.title.as_deref(),
                parsed_genre,
                params.artist.as_deref(),
            )
        };

        if !matches {
            continue;
        }

        for sheet in &song.sheets {
            let Some(sheet_chart_type) = parse_sheet_chart_type(&sheet.chart_type) else {
                continue;
            };
            let Some(sheet_diff_category) = parse_sheet_difficulty(&sheet.difficulty) else {
                continue;
            };

            if parsed_chart_type.is_some_and(|chart_type| chart_type != sheet_chart_type) {
                continue;
            }
            if parsed_diff_category
                .is_some_and(|diff_category| diff_category != sheet_diff_category)
            {
                continue;
            }

            let internal_level = sheet
                .internal_level
                .as_deref()
                .and_then(|value| value.trim().parse::<f32>().ok());
            let version = sheet
                .version_name
                .clone()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty());

            items.push(SongMetadataResponse {
                title: song.title.clone(),
                chart_type: sheet_chart_type,
                diff_category: sheet_diff_category,
                level: Some(sheet.level.clone()),
                internal_level,
                image_name: song.image_name.clone(),
                version,
                genre: song.genre.to_string(),
                artist: song.artist.clone(),
                aliases: song.aliases.clone(),
                region: sheet.region.clone(),
            });
        }
    }

    items
}

fn search_song_metadata_items(
    songs: &[models::SongCatalogSong],
    params: &SongMetadataSearchRequest,
) -> Result<SongMetadataSearchResponse> {
    let parsed_genre =
        match params.genre.as_deref() {
            Some(genre) => Some(genre.parse::<SongGenre>().ok().ok_or_else(|| {
                AppError::JsonError(format!("unknown song genre: {}", genre.trim()))
            })?),
            None => None,
        };
    let parsed_chart_type = match params.chart_type.as_deref() {
        Some(chart_type) => Some(parse_chart_type_query_value(chart_type).ok_or_else(|| {
            AppError::JsonError(format!(
                "invalid chart type: {} (expected STD or DX)",
                chart_type
            ))
        })?),
        None => None,
    };
    let parsed_diff_category = match params.diff_category.as_deref() {
        Some(diff_category) => Some(
            diff_category
                .parse::<DifficultyCategory>()
                .ok()
                .ok_or_else(|| {
                    AppError::JsonError(format!("invalid diff_category: {}", diff_category))
                })?,
        ),
        None => None,
    };
    let limit = params.limits.unwrap_or(20).min(100);

    let mut items = collect_song_metadata_items(
        songs,
        params,
        parsed_genre.as_ref(),
        parsed_chart_type,
        parsed_diff_category,
        false,
    );
    if items.is_empty() {
        items = collect_song_metadata_items(
            songs,
            params,
            parsed_genre.as_ref(),
            parsed_chart_type,
            parsed_diff_category,
            true,
        );
    }
    let total = items.len();
    items.truncate(limit);

    Ok(SongMetadataSearchResponse { total, items })
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
        let Ok(difficulty) = value.parse::<DifficultyCategory>() else {
            return Err(AppError::JsonError(format!(
                "invalid include_difficulties value: {}",
                value
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
        let Ok(version) = value.parse::<MaimaiVersion>() else {
            return Err(AppError::JsonError(format!(
                "invalid include_versions value: {}",
                value
            )));
        };
        parsed.insert(version);
    }

    Ok(Some(parsed))
}

fn parse_sheet_chart_type(sheet_type: &str) -> Option<ChartType> {
    sheet_type.parse::<ChartType>().ok()
}

fn parse_sheet_difficulty(difficulty: &str) -> Option<DifficultyCategory> {
    difficulty.parse::<DifficultyCategory>().ok()
}

fn parse_chart_type_query_value(value: &str) -> Option<ChartType> {
    value.parse::<ChartType>().ok()
}

fn is_intl_sheet(sheet: &models::SongCatalogChart) -> bool {
    sheet.region.intl
}

fn parse_intl_sheet_version(sheet: &models::SongCatalogChart) -> Option<MaimaiVersion> {
    if !is_intl_sheet(sheet) {
        return None;
    }
    let version_name = sheet.version_name.as_deref()?;
    version_name.parse::<MaimaiVersion>().ok()
}

fn select_random_index(len: usize) -> usize {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    (nanos % len as u128) as usize
}

pub(crate) async fn search_song_metadata(
    State(state): State<AppState>,
    Json(params): Json<SongMetadataSearchRequest>,
) -> Result<Json<SongMetadataSearchResponse>> {
    let song_data_root = state
        .song_data_root
        .read()
        .map_err(|_| AppError::IoError("Failed to read song data".to_string()))?;

    Ok(Json(search_song_metadata_items(
        song_data_root.as_slice(),
        &params,
    )?))
}

#[cfg(test)]
mod tests {
    use super::{
        SongMetadataSearchRequest, build_song_info_response, is_intl_sheet,
        parse_intl_sheet_version, search_song_metadata_items,
    };
    use models::{
        DifficultyCategory, MaimaiVersion, SongAliases, SongCatalogChart, SongCatalogSong,
        SongChartRegion, SongGenre,
    };

    #[test]
    fn intl_sheet_predicate_uses_region_flag() {
        let intl_sheet = SongCatalogChart {
            chart_type: "std".to_string(),
            difficulty: "basic".to_string(),
            level: "10".to_string(),
            version_name: Some("Splash".to_string()),
            internal_level: None,
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
            genre: SongGenre::Maimai,
            artist: "".to_string(),
            image_name: Some("cover.png".to_string()),
            aliases: SongAliases {
                en: vec!["Alias".to_string()],
                ko: vec!["별칭".to_string()],
            },
            sheets: vec![SongCatalogChart {
                chart_type: "dx".to_string(),
                difficulty: "master".to_string(),
                level: "14+".to_string(),
                version_name: Some("  Buddies Plus  ".to_string()),
                internal_level: Some("14.7".to_string()),
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
        assert_eq!(response.aliases.en, vec!["Alias".to_string()]);
        assert_eq!(response.aliases.ko, vec!["별칭".to_string()]);
        assert_eq!(response.sheets.len(), 1);

        let sheet = &response.sheets[0];
        assert_eq!(sheet.chart_type, models::ChartType::Dx);
        assert_eq!(sheet.difficulty, models::DifficultyCategory::Master);
        assert_eq!(sheet.version.as_deref(), Some("Buddies Plus"));
        assert_eq!(sheet.internal_level, Some(14.7));
        assert!(sheet.region.jp);
        assert!(!sheet.region.intl);
    }

    #[test]
    fn metadata_search_returns_multiple_rows_and_total() {
        let songs = vec![
            SongCatalogSong {
                title: "Link".to_string(),
                genre: SongGenre::Maimai,
                artist: "Artist A".to_string(),
                image_name: Some("a.png".to_string()),
                aliases: SongAliases::default(),
                sheets: vec![SongCatalogChart {
                    chart_type: "std".to_string(),
                    difficulty: "basic".to_string(),
                    level: "5".to_string(),
                    version_name: Some("CiRCLE".to_string()),
                    internal_level: Some("5.0".to_string()),
                    region: SongChartRegion {
                        jp: true,
                        intl: true,
                    },
                }],
            },
            SongCatalogSong {
                title: "Link".to_string(),
                genre: SongGenre::NiconicoVocaloid,
                artist: "Artist B".to_string(),
                image_name: Some("b.png".to_string()),
                aliases: SongAliases::default(),
                sheets: vec![SongCatalogChart {
                    chart_type: "std".to_string(),
                    difficulty: "basic".to_string(),
                    level: "6".to_string(),
                    version_name: Some("CiRCLE".to_string()),
                    internal_level: Some("6.0".to_string()),
                    region: SongChartRegion {
                        jp: true,
                        intl: true,
                    },
                }],
            },
        ];

        let response = search_song_metadata_items(
            &songs,
            &SongMetadataSearchRequest {
                title: Some("Link".to_string()),
                genre: None,
                artist: None,
                chart_type: None,
                diff_category: None,
                limits: Some(1),
            },
        )
        .expect("search should succeed");

        assert_eq!(response.total, 2);
        assert_eq!(response.items.len(), 1);
    }

    #[test]
    fn metadata_search_can_match_empty_artist_exactly() {
        let songs = vec![SongCatalogSong {
            title: "Empty Artist".to_string(),
            genre: SongGenre::Maimai,
            artist: "".to_string(),
            image_name: None,
            aliases: SongAliases::default(),
            sheets: vec![SongCatalogChart {
                chart_type: "std".to_string(),
                difficulty: "expert".to_string(),
                level: "10+".to_string(),
                version_name: Some("CiRCLE".to_string()),
                internal_level: Some("10.7".to_string()),
                region: SongChartRegion {
                    jp: false,
                    intl: true,
                },
            }],
        }];

        let response = search_song_metadata_items(
            &songs,
            &SongMetadataSearchRequest {
                title: Some("Empty Artist".to_string()),
                genre: Some("maimai".to_string()),
                artist: Some(String::new()),
                chart_type: Some("STD".to_string()),
                diff_category: Some("EXPERT".to_string()),
                limits: Some(10),
            },
        )
        .expect("search should succeed");

        assert_eq!(response.total, 1);
        assert_eq!(response.items[0].artist, "");
    }

    #[test]
    fn metadata_search_falls_back_to_case_insensitive_title() {
        let songs = vec![SongCatalogSong {
            title: "Link".to_string(),
            genre: SongGenre::Maimai,
            artist: "Artist A".to_string(),
            image_name: Some("a.png".to_string()),
            aliases: SongAliases::default(),
            sheets: vec![SongCatalogChart {
                chart_type: "std".to_string(),
                difficulty: "basic".to_string(),
                level: "5".to_string(),
                version_name: Some("CiRCLE".to_string()),
                internal_level: Some("5.0".to_string()),
                region: SongChartRegion {
                    jp: true,
                    intl: true,
                },
            }],
        }];

        let response = search_song_metadata_items(
            &songs,
            &SongMetadataSearchRequest {
                title: Some("link".to_string()),
                genre: None,
                artist: None,
                chart_type: None,
                diff_category: None,
                limits: Some(10),
            },
        )
        .expect("search should succeed");

        assert_eq!(response.total, 1);
        assert_eq!(response.items[0].title, "Link");
    }

    #[test]
    fn metadata_search_can_match_alias_case_insensitively() {
        let songs = vec![SongCatalogSong {
            title: "Official Song".to_string(),
            genre: SongGenre::Maimai,
            artist: "Artist A".to_string(),
            image_name: Some("a.png".to_string()),
            aliases: SongAliases {
                en: vec!["Test Alias".to_string()],
                ko: vec!["테스트 별칭".to_string()],
            },
            sheets: vec![SongCatalogChart {
                chart_type: "std".to_string(),
                difficulty: "basic".to_string(),
                level: "5".to_string(),
                version_name: Some("CiRCLE".to_string()),
                internal_level: Some("5.0".to_string()),
                region: SongChartRegion {
                    jp: true,
                    intl: true,
                },
            }],
        }];

        let response = search_song_metadata_items(
            &songs,
            &SongMetadataSearchRequest {
                title: Some("test alias".to_string()),
                genre: None,
                artist: None,
                chart_type: None,
                diff_category: None,
                limits: Some(10),
            },
        )
        .expect("search should succeed");

        assert_eq!(response.total, 1);
        assert_eq!(response.items[0].title, "Official Song");
        assert_eq!(
            response.items[0].aliases.ko,
            vec!["테스트 별칭".to_string()]
        );
    }

    #[test]
    fn metadata_search_keeps_exact_match_priority_over_fallback() {
        let songs = vec![
            SongCatalogSong {
                title: "Link".to_string(),
                genre: SongGenre::Maimai,
                artist: "Artist A".to_string(),
                image_name: Some("a.png".to_string()),
                aliases: SongAliases::default(),
                sheets: vec![SongCatalogChart {
                    chart_type: "std".to_string(),
                    difficulty: "basic".to_string(),
                    level: "5".to_string(),
                    version_name: Some("CiRCLE".to_string()),
                    internal_level: Some("5.0".to_string()),
                    region: SongChartRegion {
                        jp: true,
                        intl: true,
                    },
                }],
            },
            SongCatalogSong {
                title: "link".to_string(),
                genre: SongGenre::Maimai,
                artist: "Artist B".to_string(),
                image_name: Some("b.png".to_string()),
                aliases: SongAliases::default(),
                sheets: vec![SongCatalogChart {
                    chart_type: "std".to_string(),
                    difficulty: "basic".to_string(),
                    level: "6".to_string(),
                    version_name: Some("CiRCLE".to_string()),
                    internal_level: Some("6.0".to_string()),
                    region: SongChartRegion {
                        jp: true,
                        intl: true,
                    },
                }],
            },
        ];

        let response = search_song_metadata_items(
            &songs,
            &SongMetadataSearchRequest {
                title: Some("link".to_string()),
                genre: None,
                artist: None,
                chart_type: None,
                diff_category: None,
                limits: Some(10),
            },
        )
        .expect("search should succeed");

        assert_eq!(response.total, 1);
        assert_eq!(response.items[0].title, "link");
        assert_eq!(response.items[0].artist, "Artist B");
    }

    #[test]
    fn metadata_search_accepts_flexible_diff_category_query_values() {
        let songs = vec![SongCatalogSong {
            title: "Link".to_string(),
            genre: SongGenre::Maimai,
            artist: "Artist A".to_string(),
            image_name: Some("a.png".to_string()),
            aliases: SongAliases::default(),
            sheets: vec![SongCatalogChart {
                chart_type: "std".to_string(),
                difficulty: "remaster".to_string(),
                level: "13+".to_string(),
                version_name: Some("CiRCLE".to_string()),
                internal_level: Some("13.7".to_string()),
                region: SongChartRegion {
                    jp: true,
                    intl: true,
                },
            }],
        }];

        for diff_category in ["Re:MASTER", "re-master", "re master", "4"] {
            let response = search_song_metadata_items(
                &songs,
                &SongMetadataSearchRequest {
                    title: Some("Link".to_string()),
                    genre: Some("maimai".to_string()),
                    artist: Some("Artist A".to_string()),
                    chart_type: Some("STD".to_string()),
                    diff_category: Some(diff_category.to_string()),
                    limits: Some(10),
                },
            )
            .expect("search should succeed");

            assert_eq!(response.total, 1, "diff_category={diff_category}");
            assert_eq!(
                response.items[0].diff_category,
                DifficultyCategory::ReMaster
            );
        }
    }
}
