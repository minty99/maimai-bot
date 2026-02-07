use eyre::{ContextCompat, WrapErr};
use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt::Display;
use std::path::Path;

pub const USER_TIER_SPREADSHEET_ID: &str = "19jn6ZFmg_aMRXKK90y58IUQE-4P32wUC7XkwwnEs7Oo";

#[derive(Debug, Clone, Copy)]
pub struct UserTierSheetSpec {
    pub internal_level: &'static str,
    pub sheet_gid: i64,
}

pub const USER_TIER_SHEET_SPECS: &[UserTierSheetSpec] = &[
    UserTierSheetSpec {
        internal_level: "13.0",
        sheet_gid: 1_749_298_731,
    },
    UserTierSheetSpec {
        internal_level: "13.1",
        sheet_gid: 1_154_334_538,
    },
    UserTierSheetSpec {
        internal_level: "13.2",
        sheet_gid: 1_829_611_377,
    },
    UserTierSheetSpec {
        internal_level: "13.3",
        sheet_gid: 906_051_633,
    },
    UserTierSheetSpec {
        internal_level: "13.4",
        sheet_gid: 273_610_954,
    },
    UserTierSheetSpec {
        internal_level: "13.5",
        sheet_gid: 2_135_528_050,
    },
    UserTierSheetSpec {
        internal_level: "13.6",
        sheet_gid: 1_968_583_985,
    },
    UserTierSheetSpec {
        internal_level: "13.7",
        sheet_gid: 1_402_010_493,
    },
    UserTierSheetSpec {
        internal_level: "13.8",
        sheet_gid: 1_532_312_671,
    },
    UserTierSheetSpec {
        internal_level: "13.9",
        sheet_gid: 1_340_012_092,
    },
    UserTierSheetSpec {
        internal_level: "14.0",
        sheet_gid: 1_964_782_792,
    },
    UserTierSheetSpec {
        internal_level: "14.1",
        sheet_gid: 1_313_771_404,
    },
    UserTierSheetSpec {
        internal_level: "14.2",
        sheet_gid: 478_233_232,
    },
    UserTierSheetSpec {
        internal_level: "14.3",
        sheet_gid: 1_133_176_592,
    },
    UserTierSheetSpec {
        internal_level: "14.4",
        sheet_gid: 540_430_720,
    },
    UserTierSheetSpec {
        internal_level: "14.5",
        sheet_gid: 740_029_659,
    },
];

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserTierKey {
    pub title: String,
    pub chart_type: String,
    pub difficulty: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserTierValue {
    pub grade: String,
    pub source_internal_level: String,
}

impl Display for UserTierKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} ({}, {})",
            self.title, self.chart_type, self.difficulty
        )
    }
}

#[derive(Debug, Clone)]
struct TierImageEntry {
    grade: String,
    image_url: String,
    border_hint: BorderHint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BorderHint {
    None,
    Expert,
    ReMaster,
    Std,
    Dx,
    NewSong,
}

#[derive(Debug, Clone)]
struct CoverFingerprint {
    title: String,
    vector: Vec<f32>,
}

#[derive(Debug, Deserialize)]
struct ValuesResponse {
    #[serde(default)]
    values: Vec<Vec<Value>>,
}

#[derive(Debug, Deserialize)]
struct SpreadsheetMetaResponse {
    #[serde(default)]
    sheets: Vec<SpreadsheetMetaSheet>,
}

#[derive(Debug, Deserialize)]
struct SpreadsheetMetaSheet {
    properties: Option<SpreadsheetMetaProperties>,
}

#[derive(Debug, Deserialize)]
struct SpreadsheetMetaProperties {
    #[serde(rename = "sheetId")]
    sheet_id: Option<i64>,
    title: Option<String>,
}

pub async fn fetch_user_tier_map_for_sheet(
    client: &reqwest::Client,
    google_api_key: &str,
    spreadsheet_id: &str,
    sheet_gid: i64,
    internal_level: &str,
    song_data: &models::SongDataRoot,
    cover_dir: &Path,
) -> eyre::Result<HashMap<UserTierKey, String>> {
    let entries = fetch_sheet_entries(client, google_api_key, spreadsheet_id, sheet_gid).await?;
    let covers = build_cover_fingerprints(song_data, cover_dir)?;

    let mut map = HashMap::new();
    for entry in entries {
        let image_bytes = download_image(client, &entry.image_url)
            .await
            .wrap_err_with(|| format!("download user-tier image: {}", entry.image_url))?;
        let matched_title = match_cover_title(&image_bytes, &covers)?;

        if let Some(key) = resolve_key(song_data, matched_title, entry.border_hint, internal_level)
        {
            map.insert(key, entry.grade);
        }
    }

    Ok(map)
}

pub async fn fetch_user_tier_map_for_default_levels(
    client: &reqwest::Client,
    google_api_key: &str,
    song_data: &models::SongDataRoot,
    cover_dir: &Path,
) -> eyre::Result<HashMap<UserTierKey, UserTierValue>> {
    let mut out = HashMap::new();

    for spec in USER_TIER_SHEET_SPECS {
        let map = fetch_user_tier_map_for_sheet(
            client,
            google_api_key,
            USER_TIER_SPREADSHEET_ID,
            spec.sheet_gid,
            spec.internal_level,
            song_data,
            cover_dir,
        )
        .await
        .wrap_err_with(|| {
            format!(
                "fetch user tier sheet for internal {} (gid {})",
                spec.internal_level, spec.sheet_gid
            )
        })?;

        for (key, grade) in map {
            out.insert(
                key,
                UserTierValue {
                    grade,
                    source_internal_level: spec.internal_level.to_string(),
                },
            );
        }
    }

    Ok(out)
}

async fn fetch_sheet_entries(
    client: &reqwest::Client,
    google_api_key: &str,
    spreadsheet_id: &str,
    sheet_gid: i64,
) -> eyre::Result<Vec<TierImageEntry>> {
    let sheet_title = fetch_sheet_title_by_gid(client, google_api_key, spreadsheet_id, sheet_gid)
        .await
        .wrap_err("resolve sheet title from gid")?;

    let range = format!("{}!A:O", quote_sheet_title(&sheet_title));
    let encoded_range = urlencoding::encode(&range);
    let url = format!(
        "https://sheets.googleapis.com/v4/spreadsheets/{spreadsheet_id}/values/{encoded_range}"
    );

    let response = client
        .get(url)
        .query(&[("key", google_api_key), ("valueRenderOption", "FORMULA")])
        .send()
        .await
        .wrap_err("request google sheets values")?
        .error_for_status()
        .wrap_err("google sheets status")?
        .json::<ValuesResponse>()
        .await
        .wrap_err("parse google sheets values")?;

    Ok(extract_tier_entries_from_values(&response.values))
}

async fn fetch_sheet_title_by_gid(
    client: &reqwest::Client,
    google_api_key: &str,
    spreadsheet_id: &str,
    sheet_gid: i64,
) -> eyre::Result<String> {
    let url = format!("https://sheets.googleapis.com/v4/spreadsheets/{spreadsheet_id}");
    let response = client
        .get(url)
        .query(&[
            ("key", google_api_key),
            ("fields", "sheets.properties(sheetId,title)"),
        ])
        .send()
        .await
        .wrap_err("request spreadsheet metadata")?
        .error_for_status()
        .wrap_err("spreadsheet metadata status")?
        .json::<SpreadsheetMetaResponse>()
        .await
        .wrap_err("parse spreadsheet metadata")?;

    let title = response
        .sheets
        .iter()
        .filter_map(|s| s.properties.as_ref())
        .find(|p| p.sheet_id == Some(sheet_gid))
        .and_then(|p| p.title.clone())
        .wrap_err_with(|| format!("sheet gid not found: {sheet_gid}"))?;

    Ok(title)
}

fn quote_sheet_title(title: &str) -> String {
    if title
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return title.to_string();
    }

    let escaped = title.replace('\'', "''");
    format!("'{escaped}'")
}

fn extract_tier_entries_from_values(values: &[Vec<Value>]) -> Vec<TierImageEntry> {
    let mut out = Vec::new();

    for row in values {
        let Some(grade) = row.get(1).and_then(value_as_str).map(str::trim) else {
            continue;
        };
        if !is_tier_grade(grade) {
            continue;
        }

        for cell in row.iter().skip(2) {
            let Some(raw) = value_as_str(cell) else {
                continue;
            };
            let url = if let Some(parsed) = parse_image_formula(raw) {
                parsed
            } else if is_http_url(raw) {
                raw.to_string()
            } else {
                continue;
            };

            out.push(TierImageEntry {
                grade: grade.to_string(),
                image_url: url,
                border_hint: BorderHint::None,
            });
        }
    }

    out
}

fn value_as_str(value: &Value) -> Option<&str> {
    match value {
        Value::String(s) => Some(s.as_str()),
        _ => None,
    }
}

fn is_tier_grade(value: &str) -> bool {
    matches!(
        value,
        "S" | "A+"
            | "A"
            | "A-"
            | "B+"
            | "B"
            | "B-"
            | "C+"
            | "C"
            | "C-"
            | "D+"
            | "D"
            | "D-"
            | "E+"
            | "E"
            | "E-"
            | "F"
    )
}

fn parse_image_formula(formula: &str) -> Option<String> {
    let trimmed = formula.trim();
    if !trimmed.to_ascii_uppercase().starts_with("=IMAGE(") {
        return None;
    }

    let content = trimmed.strip_prefix("=IMAGE(")?.strip_suffix(')')?.trim();
    if !content.starts_with('"') {
        return None;
    }

    let rest = &content[1..];
    let end_quote = rest.find('"')?;
    let url = &rest[..end_quote];
    if is_http_url(url) {
        Some(url.to_string())
    } else {
        None
    }
}

fn is_http_url(value: &str) -> bool {
    value.starts_with("https://") || value.starts_with("http://")
}

async fn download_image(client: &reqwest::Client, image_url: &str) -> eyre::Result<Vec<u8>> {
    let bytes = client
        .get(image_url)
        .send()
        .await
        .wrap_err("request image")?
        .error_for_status()
        .wrap_err("image status")?
        .bytes()
        .await
        .wrap_err("image bytes")?;
    Ok(bytes.to_vec())
}

fn build_cover_fingerprints(
    song_data: &models::SongDataRoot,
    cover_dir: &Path,
) -> eyre::Result<Vec<CoverFingerprint>> {
    let mut out = Vec::new();

    for song in &song_data.songs {
        let Some(image_name) = song.image_name.as_deref() else {
            continue;
        };
        let path = cover_dir.join(image_name);
        if !path.exists() {
            continue;
        }

        let bytes = std::fs::read(&path)
            .wrap_err_with(|| format!("read cover image: {}", path.display()))?;
        let vector = image_to_vector(&bytes, true)
            .wrap_err_with(|| format!("decode cover image: {}", path.display()))?;
        out.push(CoverFingerprint {
            title: song.title.clone(),
            vector,
        });
    }

    Ok(out)
}

fn match_cover_title<'a>(
    image_bytes: &[u8],
    covers: &'a [CoverFingerprint],
) -> eyre::Result<&'a str> {
    let border_hint = classify_border_hint(image_bytes).unwrap_or(BorderHint::None);
    let query_vector = image_to_vector(image_bytes, true)?;

    let mut best: Option<(&str, f32)> = None;
    for cover in covers {
        let score = l1_distance(&query_vector, &cover.vector);
        match best {
            Some((_, best_score)) if score >= best_score => {}
            _ => best = Some((cover.title.as_str(), score)),
        }
    }

    let (title, score) = best.wrap_err("no cover candidates")?;
    if score > 13.0 {
        return Err(eyre::eyre!("no reliable cover match, score={score:.3}"));
    }

    let _ = border_hint;
    Ok(title)
}

fn image_to_vector(image_bytes: &[u8], crop_border: bool) -> eyre::Result<Vec<f32>> {
    let mut image = image::load_from_memory(image_bytes).wrap_err("decode image bytes")?;
    if crop_border {
        image = crop_center_without_border(&image);
    }

    let gray = image
        .resize_exact(32, 32, FilterType::CatmullRom)
        .grayscale()
        .to_luma8();

    let mut vector = Vec::with_capacity((gray.width() * gray.height()) as usize);
    for pixel in gray.pixels() {
        vector.push(pixel[0] as f32 / 255.0);
    }

    normalize_vector(&mut vector);
    Ok(vector)
}

fn crop_center_without_border(image: &DynamicImage) -> DynamicImage {
    let (w, h) = image.dimensions();
    let border = ((w.min(h) as f32) * 0.09).round() as u32;
    if border == 0 || border * 2 >= w || border * 2 >= h {
        return image.clone();
    }
    image.crop_imm(border, border, w - border * 2, h - border * 2)
}

fn normalize_vector(vector: &mut [f32]) {
    if vector.is_empty() {
        return;
    }

    let mean = vector.iter().sum::<f32>() / vector.len() as f32;
    let variance = vector
        .iter()
        .map(|v| {
            let d = *v - mean;
            d * d
        })
        .sum::<f32>()
        / vector.len() as f32;
    let std = variance.sqrt().max(1e-6);

    for v in vector {
        *v = (*v - mean) / std;
    }
}

fn l1_distance(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).abs())
        .sum::<f32>()
        / a.len().max(1) as f32
}

fn classify_border_hint(image_bytes: &[u8]) -> eyre::Result<BorderHint> {
    let image = image::load_from_memory(image_bytes).wrap_err("decode border image")?;
    let rgb = image.to_rgb8();
    let (w, h) = rgb.dimensions();
    if w < 10 || h < 10 {
        return Ok(BorderHint::None);
    }

    let b = ((w.min(h) as f32) * 0.08).max(1.0) as u32;
    let mut samples = Vec::new();

    for y in 0..h {
        for x in 0..w {
            if x < b || y < b || x + b >= w || y + b >= h {
                samples.push(rgb.get_pixel(x, y).0);
            }
        }
    }

    if samples.is_empty() {
        return Ok(BorderHint::None);
    }

    let (mut r, mut g, mut bl) = (0f32, 0f32, 0f32);
    for px in &samples {
        r += px[0] as f32;
        g += px[1] as f32;
        bl += px[2] as f32;
    }
    let n = samples.len() as f32;
    r /= n;
    g /= n;
    bl /= n;

    let hint = if r > 150.0 && r > g * 1.28 && r > bl * 1.28 {
        BorderHint::Expert
    } else if r > 120.0 && bl > 120.0 && g < 115.0 {
        BorderHint::ReMaster
    } else if g > 145.0 && r < 180.0 && bl < 170.0 {
        BorderHint::Dx
    } else if bl > 145.0 && r < 140.0 && g < 190.0 {
        BorderHint::Std
    } else if r > 170.0 && g > 160.0 && bl < 120.0 {
        BorderHint::NewSong
    } else {
        BorderHint::None
    };

    Ok(hint)
}

fn resolve_key(
    song_data: &models::SongDataRoot,
    matched_title: &str,
    border_hint: BorderHint,
    internal_level: &str,
) -> Option<UserTierKey> {
    let song = song_data.songs.iter().find(|s| s.title == matched_title)?;

    let mut candidates: Vec<(&str, &str)> = song
        .sheets
        .iter()
        .filter(|sheet| sheet.internal_level.as_deref() == Some(internal_level))
        .filter_map(|sheet| {
            let chart_type = normalize_chart_type(&sheet.sheet_type)?;
            let difficulty = normalize_difficulty(&sheet.difficulty)?;
            Some((chart_type, difficulty))
        })
        .collect();

    if candidates.is_empty() {
        return None;
    }

    match border_hint {
        BorderHint::Expert => candidates.retain(|(_, diff)| *diff == "EXPERT"),
        BorderHint::ReMaster => candidates.retain(|(_, diff)| *diff == "Re:MASTER"),
        BorderHint::Std => candidates.retain(|(chart, _)| *chart == "STD"),
        BorderHint::Dx => candidates.retain(|(chart, _)| *chart == "DX"),
        _ => {}
    }

    if candidates.is_empty() {
        return None;
    }

    if candidates.len() > 1 {
        if let Some(master) = candidates.iter().find(|(_, diff)| *diff == "MASTER") {
            return Some(UserTierKey {
                title: song.title.clone(),
                chart_type: master.0.to_string(),
                difficulty: master.1.to_string(),
            });
        }
    }

    let (chart_type, difficulty) = candidates[0];
    Some(UserTierKey {
        title: song.title.clone(),
        chart_type: chart_type.to_string(),
        difficulty: difficulty.to_string(),
    })
}

fn normalize_chart_type(sheet_type: &str) -> Option<&'static str> {
    match sheet_type.trim().to_ascii_lowercase().as_str() {
        "std" => Some("STD"),
        "dx" => Some("DX"),
        _ => None,
    }
}

fn normalize_difficulty(difficulty: &str) -> Option<&'static str> {
    match difficulty.trim().to_ascii_lowercase().as_str() {
        "expert" => Some("EXPERT"),
        "master" => Some("MASTER"),
        "remaster" => Some("Re:MASTER"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};
    use std::path::PathBuf;

    const LIVE_TEST_INTERNAL_LEVEL: &str = "13.0";
    const LIVE_TEST_SPREADSHEET_ID: &str = "19jn6ZFmg_aMRXKK90y58IUQE-4P32wUC7XkwwnEs7Oo";
    const LIVE_TEST_SHEET_GID: i64 = 1_749_298_731;

    fn to_png_bytes(image: ImageBuffer<Rgb<u8>, Vec<u8>>) -> Vec<u8> {
        let mut out = Vec::new();
        let dyn_img = DynamicImage::ImageRgb8(image);
        dyn_img
            .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
            .unwrap();
        out
    }

    fn solid_with_border(core: [u8; 3], border: [u8; 3]) -> Vec<u8> {
        let mut img = ImageBuffer::from_pixel(80, 80, Rgb(core));
        let b = 8;
        for y in 0..80 {
            for x in 0..80 {
                if x < b || y < b || x + b >= 80 || y + b >= 80 {
                    img.put_pixel(x, y, Rgb(border));
                }
            }
        }
        to_png_bytes(img)
    }

    #[test]
    fn parse_image_formula_extracts_url() {
        let formula = r#"=IMAGE("https://example.com/a.png",1)"#;
        assert_eq!(
            parse_image_formula(formula),
            Some("https://example.com/a.png".to_string())
        );
        assert_eq!(parse_image_formula("=SUM(A1:A3)"), None);
    }

    #[test]
    fn classify_border_color_hint() {
        let red = solid_with_border([220, 220, 220], [240, 30, 30]);
        let purple = solid_with_border([220, 220, 220], [175, 70, 180]);
        let green = solid_with_border([220, 220, 220], [120, 235, 90]);
        let blue = solid_with_border([220, 220, 220], [55, 110, 230]);

        assert_eq!(classify_border_hint(&red).unwrap(), BorderHint::Expert);
        assert_eq!(classify_border_hint(&purple).unwrap(), BorderHint::ReMaster);
        assert_eq!(classify_border_hint(&green).unwrap(), BorderHint::Dx);
        assert_eq!(classify_border_hint(&blue).unwrap(), BorderHint::Std);
    }

    #[test]
    fn resolve_key_prefers_hint_and_master() {
        let root = models::SongDataRoot {
            songs: vec![models::SongDataSong {
                title: "Song A".to_string(),
                version: None,
                image_name: Some("a.png".to_string()),
                sheets: vec![
                    models::SongDataSheet {
                        sheet_type: "std".to_string(),
                        difficulty: "expert".to_string(),
                        level: "13".to_string(),
                        internal_level: Some("13.0".to_string()),
                        user_level: None,
                    },
                    models::SongDataSheet {
                        sheet_type: "std".to_string(),
                        difficulty: "master".to_string(),
                        level: "13+".to_string(),
                        internal_level: Some("13.0".to_string()),
                        user_level: None,
                    },
                ],
            }],
        };

        let key_default =
            resolve_key(&root, "Song A", BorderHint::None, LIVE_TEST_INTERNAL_LEVEL).unwrap();
        assert_eq!(key_default.difficulty, "MASTER");

        let key_expert = resolve_key(
            &root,
            "Song A",
            BorderHint::Expert,
            LIVE_TEST_INTERNAL_LEVEL,
        )
        .unwrap();
        assert_eq!(key_expert.difficulty, "EXPERT");
    }

    #[test]
    fn cover_match_tolerates_border_variant() {
        let cover = solid_with_border([60, 120, 200], [60, 120, 200]);
        let query = solid_with_border([60, 120, 200], [240, 40, 40]);

        let cover_fp = CoverFingerprint {
            title: "Song A".to_string(),
            vector: image_to_vector(&cover, true).unwrap(),
        };

        let covers = [cover_fp];
        let matched = match_cover_title(&query, &covers).unwrap();
        assert_eq!(matched, "Song A");
    }

    #[tokio::test]
    #[ignore]
    async fn fetch_user_tier_map_for_sheet_live() {
        dotenvy::dotenv().ok();
        let api_key = std::env::var("GOOGLE_API_KEY").expect("GOOGLE_API_KEY is required");
        let client = reqwest::Client::new();

        let song_data_json = std::env::var("USER_TIER_TEST_SONG_DATA_JSON")
            .unwrap_or_else(|_| "data/song_data/data.json".to_string());
        let cover_dir = std::env::var("USER_TIER_TEST_COVER_DIR")
            .unwrap_or_else(|_| "data/song_data/cover".to_string());

        let song_data_bytes = std::fs::read(&song_data_json)
            .unwrap_or_else(|e| panic!("failed to read {song_data_json}: {e}"));
        let song_data: models::SongDataRoot = serde_json::from_slice(&song_data_bytes)
            .unwrap_or_else(|e| panic!("failed to parse {song_data_json}: {e}"));
        let cover_dir_path = PathBuf::from(cover_dir);

        let map = fetch_user_tier_map_for_sheet(
            &client,
            &api_key,
            USER_TIER_SPREADSHEET_ID,
            USER_TIER_SHEET_SPECS[0].sheet_gid,
            USER_TIER_SHEET_SPECS[0].internal_level,
            &song_data,
            &cover_dir_path,
        )
        .await
        .expect("failed to fetch live user tier map");

        for (key, value) in map.iter() {
            println!("{}: {}", key, value);
        }

        assert!(!map.is_empty());
    }
}
