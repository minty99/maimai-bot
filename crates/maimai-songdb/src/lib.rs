#![allow(dead_code)]

use eyre::{ContextCompat, WrapErr};
use models::{
    ChartType, DifficultyCategory, SongDataIndex, SongDataRoot, SongDataSheet, SongDataSong,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fmt;
use std::path::Path;

mod internal_levels;
mod user_tiers;

use internal_levels::{InternalLevelKey, InternalLevelRow};
use user_tiers::{UserTierKey, UserTierValue};

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
struct SongRow {
    song_id: String,
    category: Option<String>,
    title: String,
    artist: Option<String>,
    image_name: String,
    image_url: String,
    version: Option<String>,
    release_date: Option<String>,
    sort_order: Option<i64>,
    is_new: bool,
    is_locked: bool,
    comment: Option<String>,
}

#[derive(Debug, Clone)]
struct SheetRow {
    song_id: String,
    sheet_type: ChartType,
    difficulty: DifficultyCategory,
    level: String,
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

#[derive(Debug, Clone)]
pub struct SongDatabase {
    songs: Vec<SongRow>,
    sheets: Vec<SheetRow>,
    internal_levels: HashMap<InternalLevelKey, InternalLevelRow>,
    user_tiers: HashMap<UserTierKey, UserTierValue>,
}

impl SongDatabase {
    pub async fn fetch(config: &SongDbConfig, song_data_dir: &Path) -> eyre::Result<Self> {
        // NOTE: maimaidx.jp sometimes has SSL certificate issues ("unable to get local issuer certificate").
        // We bypass verification here since we're only fetching public cover images.
        let client = reqwest::Client::builder()
            .user_agent(&config.user_agent)
            .danger_accept_invalid_certs(true)
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

        tracing::info!("Fetching internal levels...");
        let internal_level_cache_dir = song_data_dir.join("internal_level");
        let internal_levels = internal_levels::fetch_internal_levels(
            &client,
            &config.google_api_key,
            &internal_level_cache_dir,
        )
        .await
        .wrap_err("fetch internal levels")?;

        tracing::info!("Downloading covers...");
        let cover_dir = song_data_dir.join("cover");
        download_cover_images(&client, &songs, &cover_dir).await?;

        tracing::info!("Fetching user tiers...");
        let seed_data_root = build_data_root(&songs, &sheets, &internal_levels, None);
        let user_tiers = user_tiers::fetch_user_tier_map_for_default_levels(
            &client,
            &config.google_api_key,
            &seed_data_root,
            &cover_dir,
        )
        .await?;

        Ok(SongDatabase {
            songs,
            sheets,
            internal_levels,
            user_tiers,
        })
    }

    pub fn into_data_root(self) -> eyre::Result<SongDataRoot> {
        Ok(build_data_root(
            &self.songs,
            &self.sheets,
            &self.internal_levels,
            Some(&self.user_tiers),
        ))
    }

    pub fn into_index(self) -> eyre::Result<SongDataIndex> {
        let data_root = self.into_data_root()?;
        Ok(SongDataIndex::from_root(data_root))
    }
}

fn build_data_root(
    songs: &[SongRow],
    sheets: &[SheetRow],
    internal_levels: &HashMap<InternalLevelKey, InternalLevelRow>,
    user_tiers: Option<&HashMap<UserTierKey, UserTierValue>>,
) -> SongDataRoot {
    use std::collections::BTreeMap;

    let mut song_map: BTreeMap<String, SongDataSong> = BTreeMap::new();

    for song in songs {
        song_map.insert(
            song.song_id.clone(),
            SongDataSong {
                title: song.title.clone(),
                version: song.version.clone(),
                image_name: Some(song.image_name.clone()),
                sheets: Vec::new(),
            },
        );
    }

    for sheet in sheets {
        let song = match song_map.get_mut(&sheet.song_id) {
            Some(song) => song,
            None => continue,
        };

        let il_key: InternalLevelKey = (sheet.song_id.clone(), sheet.sheet_type, sheet.difficulty);

        let internal_level = internal_levels
            .get(&il_key)
            .map(|il| il.internal_level.trim().to_string());

        let user_key = user_tiers.map(|_| UserTierKey {
            title: song.title.clone(),
            chart_type: sheet.sheet_type,
            difficulty: sheet.difficulty,
        });
        let user_tier = user_key
            .as_ref()
            .and_then(|k| user_tiers.and_then(|map| map.get(k)));

        if let (Some(internal), Some(user_tier_value)) = (internal_level.as_deref(), user_tier) {
            if internal != user_tier_value.source_internal_level {
                tracing::warn!(
                    title = %song.title,
                    chart_type = %sheet.sheet_type,
                    difficulty = %sheet.difficulty.as_str(),
                    chart_internal_level = %internal,
                    user_tier_internal_level = %user_tier_value.source_internal_level,
                    "user tier internal level mismatch"
                );
            }
        }

        song.sheets.push(SongDataSheet {
            sheet_type: sheet.sheet_type.as_lowercase().to_string(),
            difficulty: sheet.difficulty.as_lowercase().to_string(),
            level: sheet.level.clone(),
            internal_level,
            user_level: user_tier.map(|v| v.grade.clone()),
        });
    }

    SongDataRoot {
        songs: song_map.into_values().collect(),
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
    let (filtered, dropped_count) = filter_out_utage_entries(raw_songs);
    if dropped_count > 0 {
        tracing::info!(
            "Skipped {} utage entries from official songs JSON",
            dropped_count
        );
    }
    Ok(filtered)
}

fn filter_out_utage_entries(raw_songs: Vec<RawSong>) -> (Vec<RawSong>, usize) {
    let before = raw_songs.len();
    let filtered = raw_songs
        .into_iter()
        .filter(|song| song.lev_utage.is_none())
        .collect::<Vec<_>>();
    let dropped_count = before.saturating_sub(filtered.len());
    (filtered, dropped_count)
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

    let candidates: [(ChartType, DifficultyCategory, Option<&str>); 10] = [
        (
            ChartType::Dx,
            DifficultyCategory::Basic,
            raw_song.dx_lev_bas.as_deref(),
        ),
        (
            ChartType::Dx,
            DifficultyCategory::Advanced,
            raw_song.dx_lev_adv.as_deref(),
        ),
        (
            ChartType::Dx,
            DifficultyCategory::Expert,
            raw_song.dx_lev_exp.as_deref(),
        ),
        (
            ChartType::Dx,
            DifficultyCategory::Master,
            raw_song.dx_lev_mas.as_deref(),
        ),
        (
            ChartType::Dx,
            DifficultyCategory::ReMaster,
            raw_song.dx_lev_remas.as_deref(),
        ),
        (
            ChartType::Std,
            DifficultyCategory::Basic,
            raw_song.lev_bas.as_deref(),
        ),
        (
            ChartType::Std,
            DifficultyCategory::Advanced,
            raw_song.lev_adv.as_deref(),
        ),
        (
            ChartType::Std,
            DifficultyCategory::Expert,
            raw_song.lev_exp.as_deref(),
        ),
        (
            ChartType::Std,
            DifficultyCategory::Master,
            raw_song.lev_mas.as_deref(),
        ),
        (
            ChartType::Std,
            DifficultyCategory::ReMaster,
            raw_song.lev_remas.as_deref(),
        ),
    ];

    candidates
        .into_iter()
        .filter_map(|(sheet_type, difficulty, level)| {
            let level = normalize_level(level)?;
            Some(SheetRow {
                song_id: song_id.clone(),
                sheet_type,
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
    cover_dir: &Path,
) -> eyre::Result<()> {
    std::fs::create_dir_all(cover_dir).wrap_err("create cover image dir")?;

    let total = songs.len();
    let mut downloaded_count = 0;
    let mut skipped_count = 0;
    let mut failed_downloads = Vec::new();

    for song in songs {
        let cover_path = cover_dir.join(&song.image_name);

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
    fn skips_utage_sheets() {
        let mut raw_song = raw_song_stub();
        raw_song.lev_utage = Some("14".to_string());
        raw_song.kanji = Some("協奏曲".to_string());
        let sheets = extract_sheets(&raw_song);
        assert!(sheets.is_empty());
    }

    #[test]
    fn filters_out_utage_entries() {
        let normal_song = raw_song_stub();
        let mut utage_song = raw_song_stub();
        utage_song.title = "Utage Song".to_string();
        utage_song.lev_utage = Some("14".to_string());

        let (filtered, dropped_count) = filter_out_utage_entries(vec![normal_song, utage_song]);
        assert_eq!(dropped_count, 1);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].title, "Stub");
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
