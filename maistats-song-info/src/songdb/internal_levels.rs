#![allow(dead_code)]

use eyre::WrapErr;
use models::{ChartType, DifficultyCategory};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;
use tokio::time::{Duration, sleep};

use super::{SongIdentity, SongRow, normalize_song_title_value};

#[derive(Debug, Clone, Deserialize)]
struct ExtractSpec {
    sheet_name: String,
    data_indexes: Vec<usize>,
    data_offsets: [usize; 4],
}

#[derive(Debug, Clone, Deserialize)]
struct SpreadsheetSpec {
    source_version: i64,
    spreadsheet_id: String,
    extracts: Vec<ExtractSpec>,
}

#[derive(Debug, Deserialize)]
struct TitleMappings {
    rename: HashMap<String, String>,
    skip: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ValuesResponse {
    #[serde(default)]
    values: Vec<Vec<Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct InternalLevelRow {
    pub(crate) song_identity: SongIdentity,
    pub(crate) sheet_type: ChartType,
    pub(crate) difficulty: DifficultyCategory,
    pub(crate) internal_level: String,
    pub(crate) source_version: i64,
}

static SPREADSHEETS: LazyLock<Vec<SpreadsheetSpec>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("data/spreadsheets.json"))
        .expect("failed to parse embedded spreadsheets.json")
});

static TITLE_MAPPINGS: LazyLock<TitleMappings> = LazyLock::new(|| {
    serde_json::from_str(include_str!("data/title_mappings.json"))
        .expect("failed to parse embedded title_mappings.json")
});

// Frozen historical versions live in-repo so we don't depend on mutable sheet data
// once a version is no longer current.
static FROZEN_INTERNAL_LEVEL_ROWS: LazyLock<HashMap<i64, Vec<InternalLevelRow>>> =
    LazyLock::new(|| {
        [
            (6, include_str!("data/internal_level/v6.json")),
            (7, include_str!("data/internal_level/v7.json")),
            (8, include_str!("data/internal_level/v8.json")),
            (9, include_str!("data/internal_level/v9.json")),
            (10, include_str!("data/internal_level/v10.json")),
            (11, include_str!("data/internal_level/v11.json")),
            (12, include_str!("data/internal_level/v12.json")),
        ]
        .into_iter()
        .map(|(version, json)| {
            let rows = serde_json::from_str(json).unwrap_or_else(|err| {
                panic!("failed to parse frozen internal level v{version}: {err}")
            });
            (version, rows)
        })
        .collect()
    });

struct InternalLevelTitleResolver {
    song_identity_by_title: HashMap<String, SongIdentity>,
    skipped_titles: HashSet<String>,
    rename_titles: HashMap<String, String>,
}

#[derive(Debug)]
enum TitleResolution {
    Matched(SongIdentity),
    Skipped,
    Unmatched(String),
}

#[derive(Debug)]
enum RowKeyMappingFailure {
    MissingTitle,
    SkippedTitle(String),
    UnmatchedTitle(String),
    InvalidSheetType { title: String, sheet_type: String },
    InvalidDifficulty { title: String, difficulty: String },
}

impl InternalLevelTitleResolver {
    fn new(songs: &[SongRow]) -> Self {
        let mappings = &*TITLE_MAPPINGS;
        let mut candidates_by_title: HashMap<String, Vec<SongIdentity>> = HashMap::new();

        for song in songs {
            candidates_by_title
                .entry(normalize_song_title_value(&song.identity.title))
                .or_default()
                .push(song.identity.clone());
        }

        let song_identity_by_title = candidates_by_title
            .into_iter()
            .filter_map(
                |(title, song_identities)| match song_identities.as_slice() {
                    [song_identity] => Some((title, song_identity.clone())),
                    _ => None,
                },
            )
            .collect();

        Self {
            song_identity_by_title,
            skipped_titles: mappings
                .skip
                .iter()
                .map(|title| normalize_song_title_value(title))
                .collect(),
            rename_titles: mappings
                .rename
                .iter()
                .map(|(title, renamed)| {
                    (
                        normalize_song_title_value(title),
                        normalize_song_title_value(renamed),
                    )
                })
                .collect(),
        }
    }

    fn resolve_title(&self, title: &str) -> TitleResolution {
        let title = normalize_song_title_value(title);
        if self.skipped_titles.contains(&title) {
            return TitleResolution::Skipped;
        }

        let title = self.rename_titles.get(&title).cloned().unwrap_or(title);
        match self.song_identity_by_title.get(&title) {
            Some(song_identity) => TitleResolution::Matched(song_identity.clone()),
            None => TitleResolution::Unmatched(title),
        }
    }
}

fn max_column_for_extract(extract: &ExtractSpec) -> usize {
    let max_data_index = extract.data_indexes.iter().copied().max().unwrap_or(0);
    let max_offset = *extract.data_offsets.iter().max().unwrap_or(&0);
    max_data_index + max_offset
}

async fn fetch_sheet_values(
    client: &reqwest::Client,
    spreadsheet_id: &str,
    sheet_name: &str,
    max_col_idx: usize,
    api_key: &str,
) -> eyre::Result<Vec<Vec<Value>>> {
    const MAX_RETRIES: u32 = 3;
    let end_col = col_idx_to_a1(max_col_idx);
    let range = format!("{sheet_name}!A:{end_col}");
    let encoded_range = urlencoding::encode(&range);
    let url = format!(
        "https://sheets.googleapis.com/v4/spreadsheets/{spreadsheet_id}/values/{encoded_range}"
    );

    for attempt in 0..MAX_RETRIES {
        match client
            .get(&url)
            .query(&[("key", api_key), ("valueRenderOption", "UNFORMATTED_VALUE")])
            .send()
            .await
        {
            Ok(resp) => match resp.error_for_status() {
                Ok(resp) => match resp.json::<ValuesResponse>().await {
                    Ok(parsed) => return Ok(parsed.values),
                    Err(e) => {
                        if attempt < MAX_RETRIES - 1 {
                            let delay_ms = 500 * 2_u64.pow(attempt);
                            tracing::warn!(
                                "Failed to parse sheet '{}': {}. Retrying in {}ms (attempt {}/{})",
                                sheet_name,
                                e,
                                delay_ms,
                                attempt + 1,
                                MAX_RETRIES
                            );
                            sleep(Duration::from_millis(delay_ms)).await;
                            continue;
                        }
                        return Err(e).wrap_err("parse sheets values json");
                    }
                },
                Err(e) => {
                    if attempt < MAX_RETRIES - 1 {
                        let delay_ms = 500 * 2_u64.pow(attempt);
                        tracing::warn!(
                            "Sheet '{}' request failed with status: {}. Retrying in {}ms (attempt {}/{})",
                            sheet_name,
                            e,
                            delay_ms,
                            attempt + 1,
                            MAX_RETRIES
                        );
                        sleep(Duration::from_millis(delay_ms)).await;
                        continue;
                    }
                    return Err(e).wrap_err("sheets values status");
                }
            },
            Err(e) => {
                if attempt < MAX_RETRIES - 1 {
                    let delay_ms = 500 * 2_u64.pow(attempt);
                    tracing::warn!(
                        "Connection error for sheet '{}': {}. Retrying in {}ms (attempt {}/{})",
                        sheet_name,
                        e,
                        delay_ms,
                        attempt + 1,
                        MAX_RETRIES
                    );
                    sleep(Duration::from_millis(delay_ms)).await;
                    continue;
                }
                return Err(e).wrap_err("GET sheets values");
            }
        }
    }
    unreachable!()
}

fn extract_records_from_values(
    values: &[Vec<Value>],
    spec: &ExtractSpec,
    source_version: i64,
    resolver: &InternalLevelTitleResolver,
) -> Vec<InternalLevelRow> {
    let mut out = Vec::new();

    for &data_index in &spec.data_indexes {
        let title_idx = data_index + spec.data_offsets[0];
        let type_idx = data_index + spec.data_offsets[1];
        let diff_idx = data_index + spec.data_offsets[2];
        let internal_idx = data_index + spec.data_offsets[3];

        for row in values {
            let internal = row.get(internal_idx).and_then(parse_number);
            let Some(internal) = internal.filter(|v| *v > 0.0) else {
                continue;
            };

            let title = row.get(title_idx).and_then(parse_string);
            let raw_type = row.get(type_idx).and_then(parse_string);
            let raw_diff = row.get(diff_idx).and_then(parse_string);

            let mapped_row = map_row_keys(
                title.as_deref(),
                raw_type.as_deref(),
                raw_diff.as_deref(),
                resolver,
            );
            let Ok((song_identity, sheet_type, difficulty)) = mapped_row else {
                log_unmatched_internal_level_row(
                    spec,
                    source_version,
                    title.as_deref(),
                    raw_type.as_deref(),
                    raw_diff.as_deref(),
                    internal,
                    mapped_row.expect_err("already checked error case"),
                );
                continue;
            };

            out.push(InternalLevelRow {
                song_identity,
                sheet_type,
                difficulty,
                internal_level: format!("{internal:.1}"),
                source_version,
            });
        }
    }

    out
}

fn map_row_keys(
    title: Option<&str>,
    sheet_type: Option<&str>,
    difficulty: Option<&str>,
    resolver: &InternalLevelTitleResolver,
) -> Result<(SongIdentity, ChartType, DifficultyCategory), RowKeyMappingFailure> {
    let title = title.ok_or(RowKeyMappingFailure::MissingTitle)?.trim();

    let song_identity = match resolver.resolve_title(title) {
        TitleResolution::Matched(song_identity) => song_identity,
        TitleResolution::Skipped => {
            return Err(RowKeyMappingFailure::SkippedTitle(title.to_string()));
        }
        TitleResolution::Unmatched(normalized_title) => {
            return Err(RowKeyMappingFailure::UnmatchedTitle(normalized_title));
        }
    };

    let raw_sheet_type = sheet_type
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| RowKeyMappingFailure::InvalidSheetType {
            title: title.to_string(),
            sheet_type: String::new(),
        })?;
    let sheet_type = match raw_sheet_type {
        "STD" => ChartType::Std,
        "DX" => ChartType::Dx,
        _ => {
            return Err(RowKeyMappingFailure::InvalidSheetType {
                title: title.to_string(),
                sheet_type: raw_sheet_type.to_string(),
            });
        }
    };

    let raw_difficulty = difficulty
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| RowKeyMappingFailure::InvalidDifficulty {
            title: title.to_string(),
            difficulty: String::new(),
        })?;
    let difficulty = raw_difficulty.parse::<DifficultyCategory>().map_err(|_| {
        RowKeyMappingFailure::InvalidDifficulty {
            title: title.to_string(),
            difficulty: raw_difficulty.to_string(),
        }
    })?;

    Ok((song_identity, sheet_type, difficulty))
}

fn log_unmatched_internal_level_row(
    spec: &ExtractSpec,
    source_version: i64,
    title: Option<&str>,
    sheet_type: Option<&str>,
    difficulty: Option<&str>,
    internal_level: f64,
    failure: RowKeyMappingFailure,
) {
    match failure {
        RowKeyMappingFailure::MissingTitle => tracing::debug!(
            "Skipped internal level row without title: v{} / {} (type='{}', difficulty='{}', internal_level={:.1})",
            source_version,
            spec.sheet_name,
            sheet_type.unwrap_or(""),
            difficulty.unwrap_or(""),
            internal_level,
        ),
        RowKeyMappingFailure::SkippedTitle(normalized_title) => tracing::debug!(
            "Skipped internal level row due to configured skipped title: v{} / {} (title='{}', normalized_title='{}', type='{}', difficulty='{}', internal_level={:.1})",
            source_version,
            spec.sheet_name,
            title.unwrap_or(""),
            normalized_title,
            sheet_type.unwrap_or(""),
            difficulty.unwrap_or(""),
            internal_level,
        ),
        RowKeyMappingFailure::UnmatchedTitle(normalized_title) => tracing::debug!(
            "Unmatched internal level row title: v{} / {} (title='{}', normalized_title='{}', type='{}', difficulty='{}', internal_level={:.1})",
            source_version,
            spec.sheet_name,
            title.unwrap_or(""),
            normalized_title,
            sheet_type.unwrap_or(""),
            difficulty.unwrap_or(""),
            internal_level,
        ),
        RowKeyMappingFailure::InvalidSheetType {
            title,
            sheet_type: invalid_sheet_type,
        } => tracing::debug!(
            "Invalid internal level row chart type: v{} / {} (title='{}', type='{}', difficulty='{}', internal_level={:.1})",
            source_version,
            spec.sheet_name,
            title,
            invalid_sheet_type,
            difficulty.unwrap_or(""),
            internal_level,
        ),
        RowKeyMappingFailure::InvalidDifficulty {
            title,
            difficulty: invalid_difficulty,
        } => tracing::debug!(
            "Invalid internal level row difficulty: v{} / {} (title='{}', type='{}', difficulty='{}', internal_level={:.1})",
            source_version,
            spec.sheet_name,
            title,
            sheet_type.unwrap_or(""),
            invalid_difficulty,
            internal_level,
        ),
    }
}

fn parse_string(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn parse_number(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

fn col_idx_to_a1(mut idx: usize) -> String {
    let mut out = Vec::new();
    loop {
        let rem = idx % 26;
        out.push((b'A' + rem as u8) as char);
        if idx < 26 {
            break;
        }
        idx = (idx / 26) - 1;
    }
    out.iter().rev().collect()
}

async fn fetch_rows_for_spreadsheet(
    client: &reqwest::Client,
    spreadsheet: &SpreadsheetSpec,
    google_api_key: &str,
    resolver: &InternalLevelTitleResolver,
) -> eyre::Result<(Vec<InternalLevelRow>, usize, Vec<String>)> {
    let mut rows = Vec::new();
    let mut total_sheets = 0;
    let mut failed_sheets = Vec::new();

    for extract in &spreadsheet.extracts {
        total_sheets += 1;
        let sheet_identifier = format!("v{} / {}", spreadsheet.source_version, extract.sheet_name);

        match fetch_sheet_values(
            client,
            &spreadsheet.spreadsheet_id,
            &extract.sheet_name,
            max_column_for_extract(extract),
            google_api_key,
        )
        .await
        {
            Ok(values) => {
                rows.extend(extract_records_from_values(
                    &values,
                    extract,
                    spreadsheet.source_version,
                    resolver,
                ));
            }
            Err(e) => {
                tracing::error!("Failed to fetch sheet '{}': {:#}", sheet_identifier, e);
                failed_sheets.push(sheet_identifier);
            }
        }

        sleep(Duration::from_secs(1)).await;
    }

    Ok((rows, total_sheets, failed_sheets))
}

fn load_frozen_rows(version: i64) -> Option<Vec<InternalLevelRow>> {
    FROZEN_INTERNAL_LEVEL_ROWS.get(&version).cloned()
}

pub(crate) type InternalLevelKey = (SongIdentity, ChartType, DifficultyCategory);

pub(crate) async fn fetch_internal_levels(
    client: &reqwest::Client,
    google_api_key: &str,
    songs: &[SongRow],
) -> eyre::Result<HashMap<InternalLevelKey, InternalLevelRow>> {
    let resolver = InternalLevelTitleResolver::new(songs);

    let spreadsheets = &*SPREADSHEETS;

    let latest_version = spreadsheets
        .iter()
        .map(|s| s.source_version)
        .max()
        .unwrap_or(0);

    let mut all_rows = Vec::new();
    let mut total_sheets = 0;
    let mut failed_sheets = Vec::new();

    for spreadsheet in spreadsheets {
        let version = spreadsheet.source_version;
        let is_latest = version == latest_version;

        if !is_latest {
            let frozen_rows = load_frozen_rows(version).ok_or_else(|| {
                eyre::eyre!("missing embedded frozen internal levels for v{version}")
            })?;
            tracing::info!(
                "v{}: loaded {} rows from embedded frozen data",
                version,
                frozen_rows.len()
            );
            all_rows.extend(frozen_rows);
            continue;
        }

        tracing::info!("v{}: fetching from Google Sheets (latest version)", version);

        let (rows, sheet_count, failures) =
            fetch_rows_for_spreadsheet(client, spreadsheet, google_api_key, &resolver).await?;

        total_sheets += sheet_count;
        failed_sheets.extend(failures);

        all_rows.extend(rows);
    }

    let mut result = HashMap::new();
    for row in all_rows {
        let key: InternalLevelKey = (row.song_identity.clone(), row.sheet_type, row.difficulty);
        result
            .entry(key)
            .and_modify(|existing: &mut InternalLevelRow| {
                if row.source_version > existing.source_version {
                    *existing = row.clone();
                }
            })
            .or_insert(row);
    }

    if total_sheets > 0 {
        let success_count = total_sheets - failed_sheets.len();
        tracing::info!(
            "Internal levels: fetched {} / {} sheets successfully (cached versions skipped)",
            success_count,
            total_sheets
        );
    }

    if !failed_sheets.is_empty() {
        tracing::warn!(
            "Failed to fetch {} sheets: {}",
            failed_sheets.len(),
            failed_sheets.join(", ")
        );
    }

    tracing::info!("Internal levels: {} unique entries total", result.len());

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::super::{load_manual_override_rows, load_official_rows_from_json};
    use super::*;
    use models::SongGenre;
    use std::sync::Once;
    use tracing_subscriber::EnvFilter;

    const OFFICIAL_JP_SONGS_JSON: &str =
        include_str!("../examples/maimai/official/maimai_songs.json");
    static TEST_TRACING: Once = Once::new();

    fn init_test_tracing() {
        TEST_TRACING.call_once(|| {
            tracing_subscriber::fmt()
                .with_env_filter(
                    EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")),
                )
                .with_test_writer()
                .without_time()
                .try_init()
                .ok();
        });
    }

    fn resolver_for_tests() -> InternalLevelTitleResolver {
        InternalLevelTitleResolver::new(&[SongRow {
            identity: SongIdentity::new("Some Song", SongGenre::Maimai, ""),
            image_name: "cover.png".to_string(),
            image_url: "https://example.com/cover.png".to_string(),
            release_date: None,
            sort_order: None,
            is_new: false,
            is_locked: false,
            comment: None,
        }])
    }

    fn fixture_resolver() -> InternalLevelTitleResolver {
        let (mut songs, _) =
            load_official_rows_from_json(OFFICIAL_JP_SONGS_JSON).expect("load official JP rows");
        let manual_override_rows = load_manual_override_rows().expect("load manual override rows");
        songs.extend(manual_override_rows.songs);
        InternalLevelTitleResolver::new(&songs)
    }

    #[test]
    fn col_idx_to_a1_works() {
        assert_eq!(col_idx_to_a1(0), "A");
        assert_eq!(col_idx_to_a1(25), "Z");
        assert_eq!(col_idx_to_a1(26), "AA");
        assert_eq!(col_idx_to_a1(27), "AB");
        assert_eq!(col_idx_to_a1(51), "AZ");
        assert_eq!(col_idx_to_a1(52), "BA");
    }

    #[test]
    fn extract_records_from_values_parses_numeric_internal_level() {
        let spec = ExtractSpec {
            sheet_name: "dummy".to_string(),
            data_indexes: vec![0],
            data_offsets: [0, 1, 2, 3],
        };

        let values = vec![vec![
            Value::String("Some Song".to_string()),
            Value::String("STD".to_string()),
            Value::String("MAS".to_string()),
            Value::Number(serde_json::Number::from_f64(13.7).unwrap()),
        ]];

        let rows = extract_records_from_values(&values, &spec, 13, &resolver_for_tests());
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].song_identity,
            SongIdentity::new("Some Song", SongGenre::Maimai, "")
        );
        assert_eq!(rows[0].sheet_type, ChartType::Std);
        assert_eq!(rows[0].difficulty, DifficultyCategory::Master);
        assert_eq!(rows[0].internal_level, "13.7");
        assert_eq!(rows[0].source_version, 13);
    }

    #[test]
    fn embedded_frozen_versions_cover_all_non_latest_spreadsheets() {
        let latest_version = SPREADSHEETS
            .iter()
            .map(|spreadsheet| spreadsheet.source_version)
            .max()
            .expect("at least one spreadsheet");
        let expected_versions = SPREADSHEETS
            .iter()
            .map(|spreadsheet| spreadsheet.source_version)
            .filter(|version| *version != latest_version)
            .collect::<HashSet<_>>();
        let embedded_versions = FROZEN_INTERNAL_LEVEL_ROWS
            .keys()
            .copied()
            .collect::<HashSet<_>>();

        assert_eq!(embedded_versions, expected_versions);
        assert!(
            load_frozen_rows(latest_version).is_none(),
            "latest version should stay runtime-fetched"
        );
    }

    #[test]
    fn extract_records_from_values_allows_empty_title_song() {
        let spec = ExtractSpec {
            sheet_name: "dummy".to_string(),
            data_indexes: vec![0],
            data_offsets: [0, 1, 2, 3],
        };
        let resolver = InternalLevelTitleResolver::new(&[SongRow {
            identity: SongIdentity::new("", SongGenre::PopsAnime, "x0o0x_"),
            image_name: "cover.png".to_string(),
            image_url: "https://example.com/cover.png".to_string(),
            release_date: None,
            sort_order: None,
            is_new: false,
            is_locked: false,
            comment: None,
        }]);

        let values = vec![vec![
            Value::String(String::new()),
            Value::String("DX".to_string()),
            Value::String("MAS".to_string()),
            Value::Number(serde_json::Number::from_f64(12.5).unwrap()),
        ]];

        let rows = extract_records_from_values(&values, &spec, 6, &resolver);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].song_identity.title, "");
        assert_eq!(rows[0].sheet_type, ChartType::Dx);
        assert_eq!(rows[0].difficulty, DifficultyCategory::Master);
    }

    #[test]
    fn parse_string_converts_numbers_to_strings() {
        assert_eq!(
            parse_string(&Value::Number(serde_json::Number::from(13))),
            Some("13".to_string())
        );
        assert_eq!(
            parse_string(&Value::String("STD".to_string())),
            Some("STD".to_string())
        );
    }

    #[test]
    fn resolver_applies_punctuation_title_mappings() {
        let songs = vec![
            SongRow {
                identity: SongIdentity::new(
                    "Love's Theme of BADASS ～バッド・アス 愛のテーマ～",
                    SongGenre::GameVariety,
                    "",
                ),
                image_name: "love.png".to_string(),
                image_url: "https://example.com/love.png".to_string(),
                release_date: None,
                sort_order: None,
                is_new: false,
                is_locked: false,
                comment: None,
            },
            SongRow {
                identity: SongIdentity::new("Party 4U ”holy nite mix”", SongGenre::GameVariety, ""),
                image_name: "party.png".to_string(),
                image_url: "https://example.com/party.png".to_string(),
                release_date: None,
                sort_order: None,
                is_new: false,
                is_locked: false,
                comment: None,
            },
            SongRow {
                identity: SongIdentity::new("Boys O’Clock", SongGenre::Maimai, ""),
                image_name: "boys.png".to_string(),
                image_url: "https://example.com/boys.png".to_string(),
                release_date: None,
                sort_order: None,
                is_new: false,
                is_locked: false,
                comment: None,
            },
            SongRow {
                identity: SongIdentity::new("Tic Tac DREAMIN’", SongGenre::OngekiChunithm, ""),
                image_name: "tic.png".to_string(),
                image_url: "https://example.com/tic.png".to_string(),
                release_date: None,
                sort_order: None,
                is_new: false,
                is_locked: false,
                comment: None,
            },
            SongRow {
                identity: SongIdentity::new("L'épilogue", SongGenre::Maimai, ""),
                image_name: "lepilogue.png".to_string(),
                image_url: "https://example.com/lepilogue.png".to_string(),
                release_date: None,
                sort_order: None,
                is_new: false,
                is_locked: false,
                comment: None,
            },
        ];
        let resolver = InternalLevelTitleResolver::new(&songs);

        assert!(matches!(
            resolver.resolve_title("Love’s Theme of BADASS ～バッド・アス 愛のテーマ～"),
            TitleResolution::Matched(_)
        ));
        assert!(matches!(
            resolver.resolve_title("Party 4U \"holy nite mix\""),
            TitleResolution::Matched(_)
        ));
        assert!(matches!(
            resolver.resolve_title("Boys O'Clock"),
            TitleResolution::Matched(_)
        ));
        assert!(matches!(
            resolver.resolve_title("Tic Tac DREAMIN'"),
            TitleResolution::Matched(_)
        ));
        assert!(matches!(
            resolver.resolve_title("L'epilogue"),
            TitleResolution::Matched(_)
        ));
    }

    async fn fetch_rows_for_version(source_version: i64) -> Vec<InternalLevelRow> {
        init_test_tracing();
        dotenvy::dotenv().ok();
        let api_key = std::env::var("GOOGLE_API_KEY").expect("GOOGLE_API_KEY required");
        let spreadsheet = SPREADSHEETS
            .iter()
            .find(|spreadsheet| spreadsheet.source_version == source_version)
            .unwrap_or_else(|| panic!("missing spreadsheet for source_version {source_version}"));
        let client = reqwest::Client::new();
        let resolver = fixture_resolver();
        let mut all_rows = Vec::new();

        for extract in &spreadsheet.extracts {
            let values = fetch_sheet_values(
                &client,
                &spreadsheet.spreadsheet_id,
                &extract.sheet_name,
                max_column_for_extract(extract),
                &api_key,
            )
            .await
            .unwrap_or_else(|err| {
                panic!(
                    "fetch_sheet_values failed for v{} / {}: {err:#}",
                    spreadsheet.source_version, extract.sheet_name
                )
            });

            let rows = extract_records_from_values(
                &values,
                extract,
                spreadsheet.source_version,
                &resolver,
            );

            eprintln!(
                "v{} / {}: {} raw rows, {} parsed records",
                spreadsheet.source_version,
                extract.sheet_name,
                values.len(),
                rows.len()
            );
            if rows.is_empty() {
                eprintln!(
                    "v{} / {}: parsed zero records",
                    spreadsheet.source_version, extract.sheet_name
                );
            }
            all_rows.extend(rows);
        }

        eprintln!(
            "v{} total parsed records: {}",
            spreadsheet.source_version,
            all_rows.len()
        );
        all_rows
    }

    #[tokio::test]
    #[ignore]
    async fn fetch_version_6_sheets() {
        let rows = fetch_rows_for_version(6).await;
        assert!(!rows.is_empty(), "expected parsed records for version 6");
    }

    #[tokio::test]
    #[ignore]
    async fn fetch_version_7_sheets() {
        let rows = fetch_rows_for_version(7).await;
        assert!(!rows.is_empty(), "expected parsed records for version 7");
    }

    #[tokio::test]
    #[ignore]
    async fn fetch_version_8_sheets() {
        let rows = fetch_rows_for_version(8).await;
        assert!(!rows.is_empty(), "expected parsed records for version 8");
    }

    #[tokio::test]
    #[ignore]
    async fn fetch_version_9_sheets() {
        let rows = fetch_rows_for_version(9).await;
        assert!(!rows.is_empty(), "expected parsed records for version 9");
    }

    #[tokio::test]
    #[ignore]
    async fn fetch_version_10_sheets() {
        let rows = fetch_rows_for_version(10).await;
        assert!(!rows.is_empty(), "expected parsed records for version 10");
    }

    #[tokio::test]
    #[ignore]
    async fn fetch_version_11_sheets() {
        let rows = fetch_rows_for_version(11).await;
        assert!(!rows.is_empty(), "expected parsed records for version 11");
    }

    #[tokio::test]
    #[ignore]
    async fn fetch_version_12_sheets() {
        let rows = fetch_rows_for_version(12).await;
        assert!(!rows.is_empty(), "expected parsed records for version 12");
    }

    #[tokio::test]
    #[ignore]
    async fn fetch_version_13_sheets() {
        let rows = fetch_rows_for_version(13).await;
        assert!(!rows.is_empty(), "expected parsed records for version 13");
    }
}
