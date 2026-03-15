#![allow(dead_code)]

use eyre::{ContextCompat, WrapErr};
use models::{
    ChartType, DifficultyCategory, SongAliases, SongCatalog, SongCatalogChart, SongCatalogSong,
    SongChartRegion, SongGenre,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::Path;

mod aliases;
mod internal_levels;
mod manual_override;
mod sheet_versions;

use internal_levels::{InternalLevelKey, InternalLevelRow};
use manual_override::load_manual_override_rows;
use sheet_versions::SheetVersionMap;

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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub(crate) struct SongIdentity {
    pub(crate) title: String,
    pub(crate) genre: SongGenre,
    pub(crate) artist: String,
}

impl SongIdentity {
    pub(crate) fn new(title: &str, genre: SongGenre, artist: &str) -> Self {
        Self {
            title: normalize_identity_component(title),
            genre,
            artist: normalize_identity_component(artist),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SongRow {
    pub(crate) identity: SongIdentity,
    pub(crate) image_name: String,
    pub(crate) image_url: String,
    pub(crate) release_date: Option<String>,
    pub(crate) sort_order: Option<i64>,
    pub(crate) is_new: bool,
    pub(crate) is_locked: bool,
    pub(crate) comment: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct SheetRow {
    pub(crate) song_identity: SongIdentity,
    pub(crate) sheet_type: ChartType,
    pub(crate) difficulty: DifficultyCategory,
    pub(crate) level: String,
    pub(crate) source: SheetSource,
}

#[derive(Debug, Clone)]
pub(crate) enum SheetSource {
    Official,
    ManualOverride {
        version_name: String,
        internal_level: Option<String>,
        region: SongChartRegion,
    },
}

impl SheetSource {
    pub(crate) fn is_official(&self) -> bool {
        matches!(self, Self::Official)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct SheetKey<'a> {
    song_identity: &'a SongIdentity,
    chart_type: ChartType,
    difficulty: DifficultyCategory,
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
    sheet_versions: SheetVersionMap,
    internal_levels: HashMap<InternalLevelKey, InternalLevelRow>,
    aliases: HashMap<SongIdentity, SongAliases>,
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
        let manual_override_rows =
            load_manual_override_rows().wrap_err("load manual_override.json")?;
        let manual_override_aliases = manual_override_rows
            .aliases
            .iter()
            .cloned()
            .collect::<HashMap<_, _>>();
        let overridden_titles = manual_override_rows.overridden_titles.clone();
        let raw_songs = fetch_maimai_songs(&client).await?;
        let raw_songs = filter_official_songs_by_title(raw_songs, &overridden_titles)
            .wrap_err("filter official songs by manual override title")?;

        let (mut songs, mut sheets) = build_official_rows(raw_songs)?;
        songs.extend(manual_override_rows.songs);
        sheets.extend(manual_override_rows.sheets);
        ensure_unique_song_row_ids(&songs)?;
        ensure_unique_sheet_keys(&sheets)?;
        tracing::info!(
            "Processing {} songs with {} sheets",
            songs.len(),
            sheets.len()
        );

        tracing::info!("Fetching INTL sheet versions...");
        let sheet_versions = sheet_versions::fetch_intl_sheet_versions(
            &config.intl_sega_id,
            &config.intl_sega_password,
            &songs,
            &sheets,
            &overridden_titles,
        )
        .await
        .wrap_err("fetch INTL sheet versions")?;

        tracing::info!("Fetching internal levels...");
        let internal_levels =
            internal_levels::fetch_internal_levels(&client, &config.google_api_key, &songs)
                .await
                .wrap_err("fetch internal levels")?;

        tracing::info!("Fetching song aliases...");
        let aliases = aliases::fetch_song_aliases(&client)
            .await
            .unwrap_or_else(|err| {
                tracing::warn!("failed to fetch song aliases; continuing without aliases: {err:#}");
                HashMap::new()
            });
        let aliases = build_song_alias_map(
            &songs,
            aliases,
            &manual_override_aliases,
            &overridden_titles,
        );

        tracing::info!("Downloading covers...");
        let cover_dir = song_data_dir.join("cover");
        download_cover_images(&client, &songs, &cover_dir).await?;

        Ok(SongDatabase {
            songs,
            sheets,
            sheet_versions,
            internal_levels,
            aliases,
        })
    }

    pub fn into_data_root(self) -> eyre::Result<SongCatalog> {
        Ok(build_data_root(
            &self.songs,
            &self.sheets,
            &self.sheet_versions,
            &self.internal_levels,
            &self.aliases,
        ))
    }
}

fn build_data_root(
    songs: &[SongRow],
    sheets: &[SheetRow],
    sheet_versions: &SheetVersionMap,
    internal_levels: &HashMap<InternalLevelKey, InternalLevelRow>,
    aliases: &HashMap<SongIdentity, SongAliases>,
) -> SongCatalog {
    use std::collections::BTreeMap;

    let mut song_map: BTreeMap<SongIdentity, SongCatalogSong> = BTreeMap::new();

    for song in songs {
        song_map.insert(
            song.identity.clone(),
            SongCatalogSong {
                title: song.identity.title.clone(),
                genre: song.identity.genre.clone(),
                artist: song.identity.artist.clone(),
                image_name: Some(song.image_name.clone()),
                aliases: aliases.get(&song.identity).cloned().unwrap_or_default(),
                sheets: Vec::new(),
            },
        );
    }

    for sheet in sheets {
        let song = match song_map.get_mut(&sheet.song_identity) {
            Some(song) => song,
            None => continue,
        };

        let il_key: InternalLevelKey = (
            sheet.song_identity.clone(),
            sheet.sheet_type,
            sheet.difficulty,
        );

        let internal_level_from_map = internal_levels
            .get(&il_key)
            .map(|il| il.internal_level.trim().to_string());

        let (version_name, internal_level, region) = match &sheet.source {
            SheetSource::Official => {
                let version_name = sheet_versions
                    .get(&sheet.song_identity)
                    .and_then(|versions| versions.get(&sheet.sheet_type))
                    .cloned();
                (
                    version_name.clone(),
                    internal_level_from_map,
                    SongChartRegion {
                        jp: true,
                        intl: version_name.is_some(),
                    },
                )
            }
            SheetSource::ManualOverride {
                version_name,
                internal_level,
                region,
            } => (
                Some(version_name.clone()),
                internal_level.clone(),
                region.clone(),
            ),
        };

        song.sheets.push(SongCatalogChart {
            chart_type: sheet.sheet_type.as_lowercase().to_string(),
            difficulty: sheet.difficulty.as_lowercase().to_string(),
            level: sheet.level.clone(),
            version_name,
            internal_level,
            region,
        });
    }

    SongCatalog {
        songs: song_map.into_values().collect(),
    }
}

fn build_song_alias_map(
    songs: &[SongRow],
    fetched_aliases: HashMap<String, SongAliases>,
    manual_override_aliases: &HashMap<SongIdentity, SongAliases>,
    overridden_titles: &HashSet<String>,
) -> HashMap<SongIdentity, SongAliases> {
    let mut aliases_by_identity = HashMap::new();

    for song in songs {
        if let Some(aliases) = manual_override_aliases.get(&song.identity) {
            aliases_by_identity.insert(song.identity.clone(), aliases.clone());
            continue;
        }

        let normalized_title = normalize_song_title_value(&song.identity.title);
        if overridden_titles.contains(&normalized_title) {
            continue;
        }

        if let Some(aliases) = fetched_aliases.get(&song.identity.title) {
            aliases_by_identity.insert(song.identity.clone(), aliases.clone());
        }
    }

    aliases_by_identity
}

async fn fetch_maimai_songs(client: &reqwest::Client) -> eyre::Result<Vec<RawSong>> {
    let response = client
        .get(MAIMAI_SONGS_URL)
        .send()
        .await
        .wrap_err("fetch maimai songs json")?;
    let response = response
        .error_for_status()
        .wrap_err("maimai songs json status")?;
    let body = response.text().await.wrap_err("read maimai songs json")?;
    parse_maimai_songs_json(&body)
}

fn parse_maimai_songs_json(json: &str) -> eyre::Result<Vec<RawSong>> {
    let raw_songs =
        serde_json::from_str::<Vec<RawSong>>(json).wrap_err("parse maimai songs json")?;
    let (mut filtered, dropped_count) = filter_out_utage_entries(raw_songs);
    if dropped_count > 0 {
        tracing::info!(
            "Skipped {} utage entries from official songs JSON",
            dropped_count
        );
    }
    apply_jp_song_patches(&mut filtered);
    Ok(filtered)
}

fn build_official_rows(raw_songs: Vec<RawSong>) -> eyre::Result<(Vec<SongRow>, Vec<SheetRow>)> {
    ensure_unique_song_identities(&raw_songs)?;

    let songs: Vec<SongRow> = raw_songs
        .iter()
        .map(extract_song)
        .collect::<eyre::Result<Vec<_>>>()?;
    let sheets: Vec<SheetRow> = raw_songs
        .iter()
        .map(extract_sheets)
        .collect::<eyre::Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect();

    ensure_unique_song_row_ids(&songs)?;
    ensure_unique_sheet_keys(&sheets)?;

    Ok((songs, sheets))
}

/// Hardcoded patches for known JP/INTL artist discrepancies in maimai_songs.json.
/// The JP data is used as the primary source, but some songs have different artist
/// strings on INTL — which affects song identity matching against INTL score records.
/// TODO: detect these mismatches automatically in the song info server.
fn apply_jp_song_patches(songs: &mut [RawSong]) {
    for song in songs.iter_mut() {
        // "Hurtling Boys": JP artist is "さだきち イシカダス「太鼓の達人」より" but
        // INTL uses "さだきち「太鼓の達人」より". Patch to match the INTL identity.
        if song.title == "Hurtling Boys"
            && song.artist.as_deref() == Some("さだきち イシカダス「太鼓の達人」より")
        {
            song.artist = Some("さだきち「太鼓の達人」より".to_string());
        }
    }
}

fn load_official_rows_from_json(json: &str) -> eyre::Result<(Vec<SongRow>, Vec<SheetRow>)> {
    let raw_songs = parse_maimai_songs_json(json)?;
    build_official_rows(raw_songs)
}

fn filter_official_songs_by_title(
    raw_songs: Vec<RawSong>,
    ignored_titles: &HashSet<String>,
) -> eyre::Result<Vec<RawSong>> {
    if ignored_titles.is_empty() {
        return Ok(raw_songs);
    }

    raw_songs
        .into_iter()
        .filter_map(|raw_song| match derive_song_identity(&raw_song) {
            Ok(identity) => {
                let normalized_title = normalize_song_title_value(&identity.title);
                if ignored_titles.contains(&normalized_title) {
                    None
                } else {
                    Some(Ok(raw_song))
                }
            }
            Err(err) => Some(Err(err)),
        })
        .collect()
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

fn ensure_unique_song_identities(raw_songs: &[RawSong]) -> eyre::Result<()> {
    let mut seen = HashSet::new();
    let mut duplicates = Vec::new();
    for raw_song in raw_songs {
        let identity = derive_song_identity(raw_song)?;
        if !seen.insert(identity.clone()) {
            duplicates.push(identity);
        }
    }

    if !duplicates.is_empty() {
        return Err(eyre::eyre!(
            "duplicate song identity detected: {:?}",
            duplicates
        ));
    }
    Ok(())
}

fn ensure_unique_song_row_ids(songs: &[SongRow]) -> eyre::Result<()> {
    let mut seen = HashSet::new();
    let mut duplicates = Vec::new();
    for song in songs {
        if !seen.insert(song.identity.clone()) {
            duplicates.push(song.identity.clone());
        }
    }
    if !duplicates.is_empty() {
        return Err(eyre::eyre!(
            "duplicate song identity in merged songs: {:?}",
            duplicates
        ));
    }
    Ok(())
}

fn ensure_unique_sheet_keys(sheets: &[SheetRow]) -> eyre::Result<()> {
    let mut seen = HashSet::new();
    let mut duplicates = Vec::new();
    for sheet in sheets {
        let key = SheetKey {
            song_identity: &sheet.song_identity,
            chart_type: sheet.sheet_type,
            difficulty: sheet.difficulty,
        };
        if !seen.insert(key) {
            duplicates.push(format!(
                "{:?}|{}|{}",
                sheet.song_identity,
                sheet.sheet_type.as_str(),
                sheet.difficulty.as_str()
            ));
        }
    }

    if !duplicates.is_empty() {
        return Err(eyre::eyre!(
            "duplicate song/chart/difficulty key detected: {:?}",
            duplicates
        ));
    }
    Ok(())
}

fn extract_song(raw_song: &RawSong) -> eyre::Result<SongRow> {
    let identity = derive_song_identity(raw_song)?;
    let image_url = format!(
        "{}{}",
        IMAGE_BASE_URL,
        raw_song.image_url.trim_start_matches('/')
    );
    let image_name = format!("{}.png", sha256_hex(&image_url));
    let release_date = parse_release_date(raw_song.release.as_deref());
    let sort_order = raw_song.version.parse::<i64>().ok();

    Ok(SongRow {
        identity,
        image_name,
        image_url,
        release_date,
        sort_order,
        is_new: is_truthy(&raw_song.date),
        is_locked: is_truthy(&raw_song.key),
        comment: extract_comment(raw_song),
    })
}

fn extract_sheets(raw_song: &RawSong) -> eyre::Result<Vec<SheetRow>> {
    let song_identity = derive_song_identity(raw_song)?;

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

    Ok(candidates
        .into_iter()
        .filter_map(|(sheet_type, difficulty, level)| {
            let level = normalize_level(level)?;
            Some(SheetRow {
                song_identity: song_identity.clone(),
                sheet_type,
                difficulty,
                level,
                source: SheetSource::Official,
            })
        })
        .collect())
}

fn derive_song_identity(raw_song: &RawSong) -> eyre::Result<SongIdentity> {
    let genre = raw_song.catcode.parse::<SongGenre>().ok().ok_or_else(|| {
        eyre::eyre!(
            "unknown official song genre in catcode: {}",
            raw_song.catcode.trim()
        )
    })?;
    let title = normalized_song_title(raw_song, &genre);
    let artist = normalized_song_artist(raw_song.artist.as_deref());
    Ok(SongIdentity::new(&title, genre, &artist))
}

fn normalized_song_title(raw_song: &RawSong, genre: &SongGenre) -> String {
    if *genre == SongGenre::Utage && raw_song.title.trim() == "[協]青春コンプレックス" {
        if raw_song.comment.as_deref() == Some("バンドメンバーを集めて楽しもう！（入門編）")
        {
            return "[協]青春コンプレックス（入門編）".to_string();
        }
        if raw_song.comment.as_deref() == Some("バンドメンバーを集めて挑め！（ヒーロー級）")
        {
            return "[協]青春コンプレックス（ヒーロー級）".to_string();
        }
    }

    let title = normalize_identity_component(&raw_song.title);
    normalize_song_title_value(&title)
}

fn normalized_song_artist(artist: Option<&str>) -> String {
    normalize_identity_component(artist.unwrap_or_default())
}

pub(crate) fn normalize_song_title_value(title: &str) -> String {
    let title = normalize_identity_component(title);
    if title == "Bad Apple!! feat nomico" {
        "Bad Apple!! feat.nomico".to_string()
    } else {
        title
    }
}

pub(crate) fn normalize_identity_component(value: &str) -> String {
    value.trim().to_string()
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
                            song.identity.title,
                            cover_path.display(),
                            e
                        );
                        failed_downloads.push(song.identity.title.clone());
                    }
                },
                Err(e) => {
                    tracing::error!(
                        "Failed to download cover for '{}': {:#}",
                        song.identity.title,
                        e
                    );
                    failed_downloads.push(song.identity.title.clone());
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
    fn derives_song_identity_with_special_cases() {
        let mut raw_song = raw_song_stub();
        raw_song.catcode = "宴会場".to_string();
        raw_song.title = "[協]青春コンプレックス".to_string();
        raw_song.comment = Some("バンドメンバーを集めて楽しもう！（入門編）".to_string());
        assert_eq!(
            derive_song_identity(&raw_song).expect("derive song identity"),
            SongIdentity::new(
                "[協]青春コンプレックス（入門編）",
                SongGenre::Utage,
                "artist"
            )
        );

        raw_song.comment = Some("バンドメンバーを集めて挑め！（ヒーロー級）".to_string());
        assert_eq!(
            derive_song_identity(&raw_song).expect("derive song identity"),
            SongIdentity::new(
                "[協]青春コンプレックス（ヒーロー級）",
                SongGenre::Utage,
                "artist"
            )
        );

        raw_song.catcode = "niconico＆ボーカロイド".to_string();
        raw_song.title = "Link".to_string();
        raw_song.comment = None;
        assert_eq!(
            derive_song_identity(&raw_song).expect("derive song identity"),
            SongIdentity::new("Link", SongGenre::NiconicoVocaloid, "artist")
        );

        raw_song.catcode = "maimai".to_string();
        assert_eq!(
            derive_song_identity(&raw_song).expect("derive song identity"),
            SongIdentity::new("Link", SongGenre::Maimai, "artist")
        );

        raw_song.title = "Bad Apple!! feat nomico".to_string();
        assert_eq!(
            derive_song_identity(&raw_song).expect("derive song identity"),
            SongIdentity::new("Bad Apple!! feat.nomico", SongGenre::Maimai, "artist")
        );
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
        let sheets = extract_sheets(&raw_song).expect("extract sheets");
        assert!(sheets.is_empty());
    }

    #[test]
    fn derive_song_identity_returns_error_for_unknown_genre() {
        let mut raw_song = raw_song_stub();
        raw_song.catcode = "UNKNOWN".to_string();

        let err = derive_song_identity(&raw_song).expect_err("unknown genre should error");

        assert!(err.to_string().contains("unknown official song genre"));
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
    fn parses_official_maimai_songs_fixture() {
        let fixture = include_str!("../examples/maimai/official/maimai_songs.json");
        let raw_songs = parse_maimai_songs_json(fixture).expect("parse official songs fixture");
        let (songs, sheets) =
            load_official_rows_from_json(fixture).expect("extract official rows from fixture");

        assert!(
            raw_songs.len() > 1000,
            "expected JP songs fixture to contain many songs"
        );
        assert_eq!(songs.len(), raw_songs.len());
        assert!(
            sheets.len() > songs.len(),
            "expected multiple sheets across official songs"
        );
    }

    #[test]
    fn sha256_hex_produces_consistent_hash() {
        let input = "https://example.com/image.png";
        let hash1 = sha256_hex(input);
        let hash2 = sha256_hex(input);
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64);
    }

    #[test]
    fn build_data_root_sets_region_flags_for_official_and_manual_override() {
        let songs = vec![
            SongRow {
                identity: SongIdentity::new("Official Song", SongGenre::Maimai, ""),
                image_name: "official.png".to_string(),
                image_url: "https://example.com/official.png".to_string(),
                release_date: None,
                sort_order: None,
                is_new: false,
                is_locked: false,
                comment: None,
            },
            SongRow {
                identity: SongIdentity::new("Intl Song", SongGenre::Maimai, ""),
                image_name: "intl.png".to_string(),
                image_url: "https://example.com/intl.png".to_string(),
                release_date: None,
                sort_order: None,
                is_new: false,
                is_locked: false,
                comment: None,
            },
        ];
        let sheets = vec![
            SheetRow {
                song_identity: SongIdentity::new("Official Song", SongGenre::Maimai, ""),
                sheet_type: ChartType::Std,
                difficulty: DifficultyCategory::Master,
                level: "12+".to_string(),
                source: SheetSource::Official,
            },
            SheetRow {
                song_identity: SongIdentity::new("Official Song", SongGenre::Maimai, ""),
                sheet_type: ChartType::Dx,
                difficulty: DifficultyCategory::Master,
                level: "12+".to_string(),
                source: SheetSource::Official,
            },
            SheetRow {
                song_identity: SongIdentity::new("Intl Song", SongGenre::Maimai, ""),
                sheet_type: ChartType::Std,
                difficulty: DifficultyCategory::Expert,
                level: "10".to_string(),
                source: SheetSource::ManualOverride {
                    version_name: "Splash".to_string(),
                    internal_level: None,
                    region: SongChartRegion {
                        jp: false,
                        intl: true,
                    },
                },
            },
        ];
        let mut sheet_versions = SheetVersionMap::new();
        sheet_versions.insert(
            SongIdentity::new("Official Song", SongGenre::Maimai, ""),
            HashMap::from([(ChartType::Std, "Splash".to_string())]),
        );

        let catalog = build_data_root(
            &songs,
            &sheets,
            &sheet_versions,
            &HashMap::new(),
            &HashMap::new(),
        );
        let official = catalog
            .songs
            .iter()
            .find(|song| song.title == "Official Song")
            .expect("official song exists");
        let official_std = official
            .sheets
            .iter()
            .find(|sheet| sheet.chart_type == "std")
            .expect("official std exists");
        let official_dx = official
            .sheets
            .iter()
            .find(|sheet| sheet.chart_type == "dx")
            .expect("official dx exists");
        assert!(official_std.region.jp);
        assert!(official_std.region.intl);
        assert!(official_dx.region.jp);
        assert!(!official_dx.region.intl);

        let intl = catalog
            .songs
            .iter()
            .find(|song| song.title == "Intl Song")
            .expect("intl song exists");
        let intl_sheet = intl.sheets.first().expect("intl sheet exists");
        assert!(!intl_sheet.region.jp);
        assert!(intl_sheet.region.intl);
    }

    #[test]
    fn load_manual_override_rows_hashes_cover_url() {
        let rows = load_manual_override_rows().expect("load manual override rows");
        let override_song = rows
            .songs
            .iter()
            .find(|song| song.identity.title == "全世界共通リズム感テスト")
            .expect("manual override song entry exists");
        let expected = format!("{}.png", sha256_hex(&override_song.image_url));
        assert_eq!(override_song.image_name, expected);
    }

    #[test]
    fn build_song_alias_map_prefers_manual_override_aliases() {
        let songs = vec![
            SongRow {
                identity: SongIdentity::new("Link", SongGenre::Maimai, "Clean Tears feat. Youna"),
                image_name: "maimai.png".to_string(),
                image_url: "https://example.com/maimai.png".to_string(),
                release_date: None,
                sort_order: None,
                is_new: false,
                is_locked: false,
                comment: None,
            },
            SongRow {
                identity: SongIdentity::new(
                    "Link",
                    SongGenre::NiconicoVocaloid,
                    "Circle of friends(天月-あまつき-・un:c・伊東歌詞太郎・コニー・はしやん)",
                ),
                image_name: "nico.png".to_string(),
                image_url: "https://example.com/nico.png".to_string(),
                release_date: None,
                sort_order: None,
                is_new: false,
                is_locked: false,
                comment: None,
            },
        ];
        let fetched_aliases = HashMap::from([(
            "Link".to_string(),
            SongAliases {
                en: vec!["Fetched Alias".to_string()],
                ko: vec!["가져온 별칭".to_string()],
            },
        )]);
        let manual_override_aliases = HashMap::from([
            (
                songs[0].identity.clone(),
                SongAliases {
                    en: vec!["Link (maimai)".to_string()],
                    ko: vec!["링크 (마이마이)".to_string()],
                },
            ),
            (
                songs[1].identity.clone(),
                SongAliases {
                    en: vec!["Link nico".to_string()],
                    ko: vec!["링크".to_string()],
                },
            ),
        ]);
        let overridden_titles = HashSet::from([normalize_song_title_value("Link")]);

        let alias_map = build_song_alias_map(
            &songs,
            fetched_aliases,
            &manual_override_aliases,
            &overridden_titles,
        );

        assert_eq!(
            alias_map
                .get(&songs[0].identity)
                .expect("maimai aliases")
                .en,
            vec!["Link (maimai)".to_string()]
        );
        assert_eq!(
            alias_map.get(&songs[1].identity).expect("nico aliases").en,
            vec!["Link nico".to_string()]
        );
    }

    #[test]
    fn filter_official_songs_by_title_skips_manual_override_titles() {
        let manual_override_rows = load_manual_override_rows().expect("load manual override rows");
        let raw_songs = parse_maimai_songs_json(include_str!(
            "../examples/maimai/official/maimai_songs.json"
        ))
        .expect("parse fixture");

        let filtered =
            filter_official_songs_by_title(raw_songs, &manual_override_rows.overridden_titles)
                .expect("filter official songs");
        let titles = filtered
            .iter()
            .map(|song| normalize_song_title_value(&song.title))
            .collect::<HashSet<_>>();

        assert!(!titles.contains("Link"));
        assert!(!titles.contains(""));
    }

    #[test]
    fn ensure_unique_sheet_keys_detects_duplicates() {
        let sheets = vec![
            SheetRow {
                song_identity: SongIdentity::new("Song A", SongGenre::Maimai, ""),
                sheet_type: ChartType::Std,
                difficulty: DifficultyCategory::Master,
                level: "13".to_string(),
                source: SheetSource::Official,
            },
            SheetRow {
                song_identity: SongIdentity::new("Song A", SongGenre::Maimai, ""),
                sheet_type: ChartType::Std,
                difficulty: DifficultyCategory::Master,
                level: "13+".to_string(),
                source: SheetSource::ManualOverride {
                    version_name: "Splash".to_string(),
                    internal_level: None,
                    region: SongChartRegion {
                        jp: false,
                        intl: true,
                    },
                },
            },
        ];

        let result = ensure_unique_sheet_keys(&sheets);
        assert!(result.is_err());
    }

    #[test]
    fn song_identity_keeps_case_distinct() {
        let lower = SongIdentity::new("link", SongGenre::Maimai, "");
        let upper = SongIdentity::new("Link", SongGenre::Maimai, "");
        assert_ne!(lower, upper);
    }
}
