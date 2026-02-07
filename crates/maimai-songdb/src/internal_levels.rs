#![allow(dead_code)]

use eyre::WrapErr;
use models::{ChartType, DifficultyCategory};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;
use tokio::time::{sleep, Duration};

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
    pub(crate) song_id: String,
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

            let Some((song_id, sheet_type, difficulty)) = map_row_keys(title, raw_type, raw_diff)
            else {
                continue;
            };

            out.push(InternalLevelRow {
                song_id,
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
) -> Option<(String, ChartType, DifficultyCategory)> {
    let title = title?.trim();
    if title.is_empty() {
        return None;
    }

    let song_id = song_id_from_internal_level_title(title)?;

    let sheet_type = match sheet_type?.trim() {
        "STD" => ChartType::Std,
        "DX" => ChartType::Dx,
        _ => return None,
    };

    let difficulty = DifficultyCategory::from_sheet_abbreviation(difficulty?.trim())?;

    Some((song_id, sheet_type, difficulty))
}

fn parse_string(v: &Value) -> Option<&str> {
    match v {
        Value::String(s) => Some(s.as_str()),
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

fn song_id_from_internal_level_title(title: &str) -> Option<String> {
    if title == "Link" {
        return None;
    }

    let mappings = &*TITLE_MAPPINGS;

    if mappings.skip.iter().any(|s| s == title) {
        return None;
    }

    if let Some(mapped) = mappings.rename.get(title) {
        return Some(mapped.clone());
    }

    Some(title.to_string())
}

async fn fetch_rows_for_spreadsheet(
    client: &reqwest::Client,
    spreadsheet: &SpreadsheetSpec,
    google_api_key: &str,
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

fn cache_path_for_version(cache_dir: &Path, version: i64) -> std::path::PathBuf {
    cache_dir.join(format!("v{version}.json"))
}

fn load_cached_rows(path: &Path) -> eyre::Result<Vec<InternalLevelRow>> {
    let data = std::fs::read_to_string(path).wrap_err("read cached internal level file")?;
    let rows: Vec<InternalLevelRow> =
        serde_json::from_str(&data).wrap_err("parse cached internal level json")?;
    Ok(rows)
}

fn save_cached_rows(path: &Path, rows: &[InternalLevelRow]) -> eyre::Result<()> {
    let json = serde_json::to_vec_pretty(rows).wrap_err("serialize internal level rows")?;
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("tmp");
    let tmp_path = path.with_file_name(format!("{file_name}.tmp"));
    std::fs::write(&tmp_path, json).wrap_err("write temp cache file")?;
    std::fs::rename(&tmp_path, path).wrap_err("rename temp cache file")?;
    Ok(())
}

pub(crate) type InternalLevelKey = (String, ChartType, DifficultyCategory);

pub(crate) async fn fetch_internal_levels(
    client: &reqwest::Client,
    google_api_key: &str,
    cache_dir: &Path,
) -> eyre::Result<HashMap<InternalLevelKey, InternalLevelRow>> {
    std::fs::create_dir_all(cache_dir).wrap_err("create internal_level cache dir")?;

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
        let cache_file = cache_path_for_version(cache_dir, version);
        let is_latest = version == latest_version;

        if !is_latest {
            if let Ok(cached) = load_cached_rows(&cache_file) {
                tracing::info!(
                    "v{}: loaded {} rows from cache (frozen version)",
                    version,
                    cached.len()
                );
                all_rows.extend(cached);
                continue;
            }
        }

        let reason = if is_latest {
            "latest version"
        } else {
            "cache miss"
        };
        tracing::info!("v{}: fetching from Google Sheets ({reason})", version);

        let (rows, sheet_count, failures) =
            fetch_rows_for_spreadsheet(client, spreadsheet, google_api_key).await?;

        total_sheets += sheet_count;
        failed_sheets.extend(failures);

        if let Err(e) = save_cached_rows(&cache_file, &rows) {
            tracing::warn!("v{}: failed to save cache: {:#}", version, e);
        } else {
            tracing::info!(
                "v{}: saved {} rows to cache at {}",
                version,
                rows.len(),
                cache_file.display()
            );
        }

        all_rows.extend(rows);
    }

    let mut result = HashMap::new();
    for row in all_rows {
        let key: InternalLevelKey = (row.song_id.clone(), row.sheet_type, row.difficulty);
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
    use super::*;

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

        let rows = extract_records_from_values(&values, &spec, 13);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].song_id, "Some Song");
        assert_eq!(rows[0].sheet_type, ChartType::Std);
        assert_eq!(rows[0].difficulty, DifficultyCategory::Master);
        assert_eq!(rows[0].internal_level, "13.7");
        assert_eq!(rows[0].source_version, 13);
    }

    #[tokio::test]
    #[ignore]
    async fn fetch_latest_version_first_sheet() {
        dotenvy::dotenv().ok();
        let api_key = std::env::var("GOOGLE_API_KEY").expect("GOOGLE_API_KEY required");
        let spreadsheets = &*SPREADSHEETS;
        let latest = spreadsheets
            .iter()
            .max_by_key(|s| s.source_version)
            .expect("no spreadsheets defined");
        let extract = &latest.extracts[0];

        let client = reqwest::Client::new();
        let values = fetch_sheet_values(
            &client,
            &latest.spreadsheet_id,
            &extract.sheet_name,
            max_column_for_extract(extract),
            &api_key,
        )
        .await
        .expect("fetch_sheet_values failed");

        let rows = extract_records_from_values(&values, extract, latest.source_version);

        eprintln!(
            "v{} / {}: {} raw rows, {} parsed records",
            latest.source_version,
            extract.sheet_name,
            values.len(),
            rows.len()
        );
        eprintln!("rows: {:#?}", rows);
        assert!(!rows.is_empty(), "expected at least one parsed record");
    }
}
