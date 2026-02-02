#![allow(dead_code)]

use eyre::{ContextCompat, WrapErr};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

pub mod internal_levels;

pub const SONG_DATA_SUBDIR: &str = "song_data";
const MAIMAI_SONGS_URL: &str = "https://maimai.sega.jp/data/maimai_songs.json";
const IMAGE_BASE_URL: &str = "https://maimaidx.jp/maimai-mobile/img/Music/";

#[derive(Debug, Deserialize)]
struct RawSong {
    catcode: String,
    title: String,
    artist: Option<String>,
    image_url: String,
    version: String,
    #[serde(default)]
    release: Option<String>,
    #[serde(default)]
    comment: Option<String>,
    #[serde(default)]
    utage_comment: Option<String>,
    #[serde(default)]
    buddy: Option<String>,
    #[serde(default)]
    date: Option<String>,
    #[serde(default)]
    key: Option<String>,
    #[serde(default)]
    dx_lev_bas: Option<String>,
    #[serde(default)]
    dx_lev_adv: Option<String>,
    #[serde(default)]
    dx_lev_exp: Option<String>,
    #[serde(default)]
    dx_lev_mas: Option<String>,
    #[serde(default)]
    dx_lev_remas: Option<String>,
    #[serde(default)]
    lev_bas: Option<String>,
    #[serde(default)]
    lev_adv: Option<String>,
    #[serde(default)]
    lev_exp: Option<String>,
    #[serde(default)]
    lev_mas: Option<String>,
    #[serde(default)]
    lev_remas: Option<String>,
    #[serde(default)]
    lev_utage: Option<String>,
    #[serde(default)]
    kanji: Option<String>,
    #[serde(default)]
    utage_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongRow {
    pub song_id: String,
    pub category: Option<String>,
    pub title: String,
    pub artist: Option<String>,
    pub image_name: String,
    pub image_url: String,
    pub version: Option<String>,
    pub release_date: Option<String>,
    pub sort_order: Option<i64>,
    pub is_new: bool,
    pub is_locked: bool,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SheetRow {
    pub song_id: String,
    pub sheet_type: String,
    pub difficulty: String,
    pub level: String,
}

#[derive(Clone)]
pub struct SongDbConfig {
    pub intl_sega_id: String,
    pub intl_sega_password: String,
    pub jp_sega_id: String,
    pub jp_sega_password: String,
    pub user_agent: String,
    pub google_api_key: String,
}

impl fmt::Debug for SongDbConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SongDbConfig")
            .field("intl_sega_id", &"<redacted>")
            .field("intl_sega_password", &"<redacted>")
            .field("jp_sega_id", &"<redacted>")
            .field("jp_sega_password", &"<redacted>")
            .field("user_agent", &self.user_agent)
            .field("google_api_key", &"<redacted>")
            .finish()
    }
}

impl SongDbConfig {
    pub fn from_env() -> eyre::Result<Self> {
        let intl_sega_id = std::env::var("MAIMAI_INTL_SEGA_ID")
            .or_else(|_| std::env::var("SEGA_ID"))
            .wrap_err("missing env var: MAIMAI_INTL_SEGA_ID or SEGA_ID")?;
        let intl_sega_password = std::env::var("MAIMAI_INTL_SEGA_PASSWORD")
            .or_else(|_| std::env::var("SEGA_PASSWORD"))
            .wrap_err("missing env var: MAIMAI_INTL_SEGA_PASSWORD or SEGA_PASSWORD")?;
        let jp_sega_id =
            std::env::var("MAIMAI_JP_SEGA_ID").wrap_err("missing env var: MAIMAI_JP_SEGA_ID")?;
        let jp_sega_password = std::env::var("MAIMAI_JP_SEGA_PASSWORD")
            .wrap_err("missing env var: MAIMAI_JP_SEGA_PASSWORD")?;
        let user_agent = std::env::var("USER_AGENT").wrap_err("missing env var: USER_AGENT")?;
        let google_api_key =
            std::env::var("GOOGLE_API_KEY").wrap_err("missing env var: GOOGLE_API_KEY")?;

        Ok(Self {
            intl_sega_id,
            intl_sega_password,
            jp_sega_id,
            jp_sega_password,
            user_agent,
            google_api_key,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongDatabase {
    pub songs: Vec<SongRow>,
    pub sheets: Vec<SheetRow>,
    pub internal_levels: HashMap<(String, String, String), internal_levels::InternalLevelRow>,
}

impl SongDatabase {
    pub async fn fetch(config: &SongDbConfig, image_output_dir: &Path) -> eyre::Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent(&config.user_agent)
            .build()
            .wrap_err("build reqwest client")?;

        tracing::info!("Fetching official maimai songs JSON...");
        let raw_songs = fetch_maimai_songs(&client).await?;
        ensure_unique_song_ids(&raw_songs)?;

        let songs: Vec<SongRow> = raw_songs.iter().map(extract_song).collect();
        let sheets: Vec<SheetRow> = raw_songs.iter().flat_map(extract_sheets).collect();
        tracing::info!(
            "Processing {} songs with {} sheets",
            songs.len(),
            sheets.len()
        );

        tracing::info!("Fetching internal levels and downloading covers in parallel...");
        let (internal_levels_result, cover_result) = tokio::join!(
            internal_levels::fetch_internal_levels(&client, &config.google_api_key),
            download_cover_images(&client, &songs, image_output_dir)
        );

        let internal_levels = internal_levels_result?;
        cover_result?;

        tracing::info!("Completed internal levels fetch and cover downloads");

        Ok(SongDatabase {
            songs,
            sheets,
            internal_levels,
        })
    }
}

async fn fetch_maimai_songs(client: &reqwest::Client) -> eyre::Result<Vec<RawSong>> {
    let response = client
        .get(MAIMAI_SONGS_URL)
        .send()
        .await
        .wrap_err("fetch maimai songs json")?;
    let raw_songs = response
        .error_for_status()
        .wrap_err("maimai songs json status")?
        .json::<Vec<RawSong>>()
        .await
        .wrap_err("parse maimai songs json")?;
    Ok(raw_songs)
}

fn ensure_unique_song_ids(raw_songs: &[RawSong]) -> eyre::Result<()> {
    let mut seen = std::collections::HashSet::new();
    let mut duplicates = Vec::new();
    for raw_song in raw_songs {
        let song_id = derive_song_id(raw_song);
        if !seen.insert(song_id.clone()) {
            duplicates.push(song_id);
        }
    }

    if !duplicates.is_empty() {
        return Err(eyre::eyre!("duplicate song_id detected: {:?}", duplicates));
    }
    Ok(())
}

fn extract_song(raw_song: &RawSong) -> SongRow {
    let image_url = format!(
        "{}{}",
        IMAGE_BASE_URL,
        raw_song.image_url.trim_start_matches('/')
    );
    let image_name = format!("{}.png", sha256_hex(&image_url));
    let version_id = raw_song
        .version
        .get(0..3)
        .and_then(|prefix| prefix.parse::<i32>().ok())
        .unwrap_or(0);
    let version = version_from_version_id(version_id).map(str::to_string);
    let release_date = parse_release_date(raw_song.release.as_deref());
    let sort_order = raw_song.version.parse::<i64>().ok();

    let artist = raw_song
        .artist
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    SongRow {
        song_id: derive_song_id(raw_song),
        category: Some(raw_song.catcode.clone()),
        title: raw_song.title.clone(),
        artist,
        image_name,
        image_url,
        version,
        release_date,
        sort_order,
        is_new: is_truthy(&raw_song.date),
        is_locked: is_truthy(&raw_song.key),
        comment: extract_comment(raw_song),
    }
}

fn extract_sheets(raw_song: &RawSong) -> Vec<SheetRow> {
    let song_id = derive_song_id(raw_song);
    let utage_type = raw_song
        .kanji
        .as_deref()
        .or(raw_song.utage_type.as_deref())
        .unwrap_or("");

    let candidates = [
        ("dx", "basic", raw_song.dx_lev_bas.as_deref()),
        ("dx", "advanced", raw_song.dx_lev_adv.as_deref()),
        ("dx", "expert", raw_song.dx_lev_exp.as_deref()),
        ("dx", "master", raw_song.dx_lev_mas.as_deref()),
        ("dx", "remaster", raw_song.dx_lev_remas.as_deref()),
        ("std", "basic", raw_song.lev_bas.as_deref()),
        ("std", "advanced", raw_song.lev_adv.as_deref()),
        ("std", "expert", raw_song.lev_exp.as_deref()),
        ("std", "master", raw_song.lev_mas.as_deref()),
        ("std", "remaster", raw_song.lev_remas.as_deref()),
        ("utage", "utage", raw_song.lev_utage.as_deref()),
    ];

    candidates
        .iter()
        .filter_map(|(sheet_type, difficulty, level)| {
            let level = normalize_level(*level)?;
            let difficulty = if *sheet_type == "utage" {
                format!("【{}】", utage_type)
            } else {
                difficulty.to_string()
            };
            Some(SheetRow {
                song_id: song_id.clone(),
                sheet_type: sheet_type.to_string(),
                difficulty,
                level,
            })
        })
        .collect()
}

fn derive_song_id(raw_song: &RawSong) -> String {
    if raw_song.catcode == "宴会場" {
        if raw_song.title == "[協]青春コンプレックス" {
            if raw_song.comment.as_deref() == Some("バンドメンバーを集めて楽しもう！（入門編）")
            {
                return "[協]青春コンプレックス（入門編）".to_string();
            }
            if raw_song.comment.as_deref() == Some("バンドメンバーを集めて挑め！（ヒーロー級）")
            {
                return "[協]青春コンプレックス（ヒーロー級）".to_string();
            }
        }
        return raw_song.title.clone();
    }

    if raw_song.title == "Link" {
        if raw_song.catcode == "maimai" {
            return "Link".to_string();
        }
        if raw_song.catcode == "niconico＆ボーカロイド" {
            return "Link (2)".to_string();
        }
    }

    if raw_song.title == "Bad Apple!! feat nomico" {
        return "Bad Apple!! feat.nomico".to_string();
    }

    raw_song.title.clone()
}

fn extract_comment(raw_song: &RawSong) -> Option<String> {
    let mut comment = raw_song
        .comment
        .as_deref()
        .or(raw_song.utage_comment.as_deref())
        .map(str::to_string)?;
    if is_truthy(&raw_song.buddy) {
        comment = format!("【🤝バディ】{comment}");
    }
    Some(comment)
}

fn version_from_version_id(version_id: i32) -> Option<&'static str> {
    match version_id {
        0 => None,
        100 => Some("maimai"),
        110 => Some("maimai PLUS"),
        120 => Some("GreeN"),
        130 => Some("GreeN PLUS"),
        140 => Some("ORANGE"),
        150 => Some("ORANGE PLUS"),
        160 => Some("PiNK"),
        170 => Some("PiNK PLUS"),
        180 => Some("MURASAKi"),
        185 => Some("MURASAKi PLUS"),
        190 => Some("MiLK"),
        195 => Some("MiLK PLUS"),
        199 => Some("FiNALE"),
        200 => Some("maimaiでらっくす"),
        205 => Some("maimaiでらっくす PLUS"),
        210 => Some("Splash"),
        215 => Some("Splash PLUS"),
        220 => Some("UNiVERSE"),
        225 => Some("UNiVERSE PLUS"),
        230 => Some("FESTiVAL"),
        235 => Some("FESTiVAL PLUS"),
        240 => Some("BUDDiES"),
        245 => Some("BUDDiES PLUS"),
        250 => Some("PRiSM"),
        255 => Some("PRiSM PLUS"),
        260 => Some("CiRCLE"),
        _ => None,
    }
}

fn parse_release_date(value: Option<&str>) -> Option<String> {
    let value = value?.trim();
    if value.is_empty() || value == "0" {
        return None;
    }
    if value.parse::<i32>().ok() == Some(0) {
        return None;
    }
    if value.len() < 6 {
        return None;
    }
    Some(format!(
        "20{}-{}-{}",
        &value[0..2],
        &value[2..4],
        &value[4..6]
    ))
}

fn normalize_level(level: Option<&str>) -> Option<String> {
    let level = level?.trim();
    if level.is_empty() {
        None
    } else {
        Some(level.to_string())
    }
}

fn is_truthy(value: &Option<String>) -> bool {
    value.as_deref().is_some_and(|text| !text.trim().is_empty())
}

fn sha256_hex(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    let digest = hasher.finalize();
    hex::encode(digest)
}

async fn download_image(client: &reqwest::Client, image_url: &str) -> eyre::Result<Vec<u8>> {
    const MAX_RETRIES: u32 = 3;

    for attempt in 0..MAX_RETRIES {
        let result = async {
            let resp = client.get(image_url).send().await?;
            let resp = resp.error_for_status()?;
            let bytes = resp.bytes().await?;
            Ok::<_, eyre::Error>(bytes.to_vec())
        }
        .await;

        match result {
            Ok(data) => return Ok(data),
            Err(e) if attempt < MAX_RETRIES - 1 => {
                let delay_ms = 200 * 2_u64.pow(attempt);
                tracing::warn!(
                    "Failed to download '{}': {}. Retrying in {}ms (attempt {}/{})",
                    image_url,
                    e,
                    delay_ms,
                    attempt + 1,
                    MAX_RETRIES
                );
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            }
            Err(e) => return Err(e.wrap_err("fetch cover image")),
        }
    }
    unreachable!()
}

fn cover_image_path(output_dir: &Path, image_name: &str) -> PathBuf {
    output_dir.join("cover").join(image_name)
}

fn should_download(cover_path: &Path) -> bool {
    !cover_path.exists()
}

fn write_atomic(path: &Path, contents: &[u8]) -> eyre::Result<()> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .wrap_err("invalid output filename")?;
    let tmp_path = path.with_file_name(format!("{file_name}.tmp"));
    std::fs::write(&tmp_path, contents).wrap_err("write temp file")?;
    std::fs::rename(&tmp_path, path).wrap_err("rename temp file")?;
    Ok(())
}

async fn download_cover_images(
    client: &reqwest::Client,
    songs: &[SongRow],
    output_dir: &Path,
) -> eyre::Result<()> {
    let cover_dir = output_dir.join("cover");
    std::fs::create_dir_all(&cover_dir).wrap_err("create cover image dir")?;

    let total = songs.len();
    let mut downloaded_count = 0;
    let mut skipped_count = 0;
    let mut failed_downloads = Vec::new();

    for song in songs {
        let cover_path = cover_image_path(output_dir, &song.image_name);

        if should_download(&cover_path) {
            match download_image(client, &song.image_url).await {
                Ok(downloaded) => match write_atomic(&cover_path, &downloaded) {
                    Ok(_) => {
                        downloaded_count += 1;
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to write cover '{}' to '{}': {:#}",
                            song.title,
                            cover_path.display(),
                            e
                        );
                        failed_downloads.push(song.title.clone());
                    }
                },
                Err(e) => {
                    tracing::error!("Failed to download cover for '{}': {:#}", song.title, e);
                    failed_downloads.push(song.title.clone());
                }
            }
        } else {
            skipped_count += 1;
        }
    }

    tracing::info!(
        "Cover images: total {} songs, downloaded {}, skipped {}, failed {}",
        total,
        downloaded_count,
        skipped_count,
        failed_downloads.len()
    );

    if !failed_downloads.is_empty() {
        tracing::warn!(
            "Failed to download {} covers. First 10: {}",
            failed_downloads.len(),
            failed_downloads
                .iter()
                .take(10)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_song_stub() -> RawSong {
        RawSong {
            catcode: "maimai".to_string(),
            title: "Stub".to_string(),
            artist: Some("artist".to_string()),
            image_url: "dummy.png".to_string(),
            version: "24001".to_string(),
            release: Some("240101".to_string()),
            comment: None,
            utage_comment: None,
            buddy: None,
            date: None,
            key: None,
            dx_lev_bas: None,
            dx_lev_adv: None,
            dx_lev_exp: None,
            dx_lev_mas: None,
            dx_lev_remas: None,
            lev_bas: None,
            lev_adv: None,
            lev_exp: None,
            lev_mas: None,
            lev_remas: None,
            lev_utage: None,
            kanji: None,
            utage_type: None,
        }
    }

    #[test]
    fn derives_song_id_with_special_cases() {
        let mut raw_song = raw_song_stub();
        raw_song.catcode = "宴会場".to_string();
        raw_song.title = "[協]青春コンプレックス".to_string();
        raw_song.comment = Some("バンドメンバーを集めて楽しもう！（入門編）".to_string());
        assert_eq!(
            derive_song_id(&raw_song),
            "[協]青春コンプレックス（入門編）"
        );

        raw_song.comment = Some("バンドメンバーを集めて挑め！（ヒーロー級）".to_string());
        assert_eq!(
            derive_song_id(&raw_song),
            "[協]青春コンプレックス（ヒーロー級）"
        );

        raw_song.catcode = "niconico＆ボーカロイド".to_string();
        raw_song.title = "Link".to_string();
        raw_song.comment = None;
        assert_eq!(derive_song_id(&raw_song), "Link (2)");

        raw_song.catcode = "maimai".to_string();
        assert_eq!(derive_song_id(&raw_song), "Link");

        raw_song.title = "Bad Apple!! feat nomico".to_string();
        assert_eq!(derive_song_id(&raw_song), "Bad Apple!! feat.nomico");
    }

    #[test]
    fn maps_version_prefix_to_version_name() {
        let version_id = "24001"
            .get(0..3)
            .and_then(|prefix| prefix.parse::<i32>().ok());
        let version = version_id.and_then(version_from_version_id);
        assert_eq!(version, Some("BUDDiES"));

        let version_id = "25599"
            .get(0..3)
            .and_then(|prefix| prefix.parse::<i32>().ok());
        let version = version_id.and_then(version_from_version_id);
        assert_eq!(version, Some("PRiSM PLUS"));
    }

    #[test]
    fn parses_release_date_formats() {
        assert_eq!(parse_release_date(None), None);
        assert_eq!(parse_release_date(Some("")), None);
        assert_eq!(parse_release_date(Some("0")), None);
        assert_eq!(parse_release_date(Some("12345")), None);
        assert_eq!(
            parse_release_date(Some("240101")),
            Some("2024-01-01".to_string())
        );
        assert_eq!(
            parse_release_date(Some("231225")),
            Some("2023-12-25".to_string())
        );
    }

    #[test]
    fn normalize_level_handles_empty_and_whitespace() {
        assert_eq!(normalize_level(None), None);
        assert_eq!(normalize_level(Some("")), None);
        assert_eq!(normalize_level(Some("  ")), None);
        assert_eq!(normalize_level(Some("13+")), Some("13+".to_string()));
        assert_eq!(normalize_level(Some(" 14  ")), Some("14".to_string()));
    }

    #[test]
    fn extracts_sheets_with_utage_type() {
        let mut raw_song = raw_song_stub();
        raw_song.lev_utage = Some("14".to_string());
        raw_song.kanji = Some("協奏曲".to_string());
        let sheets = extract_sheets(&raw_song);
        let utage_sheet = sheets.iter().find(|s| s.sheet_type == "utage");
        assert!(utage_sheet.is_some());
        let utage = utage_sheet.unwrap();
        assert_eq!(utage.difficulty, "【協奏曲】");
        assert_eq!(utage.level, "14");
    }

    #[test]
    fn sha256_hex_produces_consistent_hash() {
        let input = "https://example.com/image.png";
        let hash1 = sha256_hex(input);
        let hash2 = sha256_hex(input);
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64);
    }
}
