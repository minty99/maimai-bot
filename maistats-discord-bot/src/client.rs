use eyre::{Result, WrapErr};
use models::{
    ChartType, DifficultyCategory, ParsedPlayerProfile, PlayRecordApiResponse, SongAliases,
    SongChartRegion, SongDetailScoreApiResponse, VersionApiResponse, is_minor_or_more_outdated,
};
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;
use tokio::sync::RwLock;
use tokio::time::{Duration, sleep};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RecordCollectorErrorResponse {
    message: String,
    code: String,
    #[serde(default)]
    maintenance: Option<bool>,
}

#[derive(Debug, Clone)]
pub(crate) struct ApiError {
    status: reqwest::StatusCode,
    code: String,
    message: String,
}

impl ApiError {
    fn from_record_collector(
        status: reqwest::StatusCode,
        error: RecordCollectorErrorResponse,
    ) -> Self {
        Self {
            status,
            code: error.code,
            message: error.message,
        }
    }

    fn from_http_text(status: reqwest::StatusCode, body: &str) -> Self {
        Self {
            status,
            code: "HTTP_ERROR".to_string(),
            message: body.trim().to_string(),
        }
    }

    pub(crate) fn code(&self) -> &str {
        &self.code
    }

    pub(crate) fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.message.is_empty() {
            write!(f, "HTTP {} [{}]", self.status, self.code)
        } else {
            write!(f, "HTTP {} [{}]: {}", self.status, self.code, self.message)
        }
    }
}

impl std::error::Error for ApiError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SongMetadata {
    pub(crate) title: String,
    pub(crate) chart_type: ChartType,
    pub(crate) diff_category: DifficultyCategory,
    pub(crate) level: Option<String>,
    pub(crate) internal_level: Option<f32>,
    pub(crate) image_name: Option<String>,
    pub(crate) version: Option<String>,
    pub(crate) genre: String,
    pub(crate) artist: String,
    pub(crate) aliases: SongAliases,
    pub(crate) region: SongChartRegion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SongCatalogSheet {
    pub(crate) chart_type: ChartType,
    #[serde(rename = "difficulty")]
    pub(crate) diff_category: DifficultyCategory,
    pub(crate) level: String,
    pub(crate) version: Option<String>,
    pub(crate) internal_level: Option<f32>,
    pub(crate) region: SongChartRegion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SongCatalogSong {
    pub(crate) title: String,
    pub(crate) genre: String,
    pub(crate) artist: String,
    pub(crate) image_name: Option<String>,
    pub(crate) aliases: SongAliases,
    pub(crate) sheets: Vec<SongCatalogSheet>,
}

#[derive(Debug, Clone)]
struct CachedSongCatalog {
    songs: Vec<SongCatalogSong>,
    fetched_at: Instant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SongMetadataSearchRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) genre: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) artist: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) chart_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) diff_category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) limits: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SongMetadataSearchResponse {
    pub(crate) total: usize,
    pub(crate) items: Vec<SongMetadata>,
}

#[derive(Debug, Clone)]
pub struct SongDatabaseClient {
    client: Client,
    base_url: String,
    cache: Arc<RwLock<Option<CachedSongCatalog>>>,
}

#[derive(Debug, Clone)]
pub struct RecordCollectorClient {
    client: Client,
    base_url: String,
}

pub(crate) const BOT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RecordCollectorVersionIssue {
    VersionMismatch,
    InvalidResponse,
    Unreachable,
}

#[derive(Debug, Clone)]
pub(crate) struct RecordCollectorVersionStatus {
    collector_version: Option<String>,
    issue: Option<RecordCollectorVersionIssue>,
}

#[derive(Debug, Clone)]
struct CachedRecordCollectorVersionStatus {
    status: RecordCollectorVersionStatus,
    fetched_at: Instant,
}

const VERSION_STATUS_CACHE_TTL: Duration = Duration::from_secs(300);
static VERSION_STATUS_CACHE: OnceLock<Mutex<HashMap<String, CachedRecordCollectorVersionStatus>>> =
    OnceLock::new();

impl RecordCollectorVersionStatus {
    pub(crate) fn compatible(collector_version: String) -> Self {
        Self {
            collector_version: Some(collector_version),
            issue: None,
        }
    }

    pub(crate) fn outdated(
        collector_version: Option<String>,
        issue: RecordCollectorVersionIssue,
    ) -> Self {
        Self {
            collector_version,
            issue: Some(issue),
        }
    }

    pub(crate) fn collector_version(&self) -> Option<&str> {
        self.collector_version.as_deref()
    }

    pub(crate) fn issue(&self) -> Option<RecordCollectorVersionIssue> {
        self.issue
    }
}

fn build_client() -> Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .wrap_err("build http client")
}

fn version_status_cache() -> &'static Mutex<HashMap<String, CachedRecordCollectorVersionStatus>> {
    VERSION_STATUS_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn normalize_record_collector_path(path: &str) -> String {
    let trimmed = path.trim_end_matches('/');
    let trimmed = if trimmed.is_empty() { "/" } else { trimmed };

    for suffix in ["/health/ready", "/api/player", "/api/version"] {
        if let Some(prefix) = trimmed.strip_suffix(suffix) {
            return if prefix.is_empty() {
                "/".to_string()
            } else {
                prefix.to_string()
            };
        }
    }

    trimmed.to_string()
}

pub(crate) fn normalize_record_collector_url(input: &str) -> Result<String> {
    let trimmed = input.trim();
    eyre::ensure!(
        !trimmed.is_empty(),
        "Please provide a record collector server URL."
    );

    let mut url = Url::parse(trimmed).wrap_err("parse record collector url")?;
    eyre::ensure!(
        matches!(url.scheme(), "http" | "https"),
        "Record collector URL must use http or https."
    );
    eyre::ensure!(
        url.host_str().is_some(),
        "Record collector URL must include a host."
    );

    let normalized_path = normalize_record_collector_path(url.path());
    url.set_path(&normalized_path);
    url.set_query(None);
    url.set_fragment(None);

    Ok(url.as_str().trim_end_matches('/').to_string())
}

fn convert_song_catalog(database: models::SongDatabase) -> Result<Vec<SongCatalogSong>> {
    database
        .songs
        .into_iter()
        .map(|song| {
            let sheets = song
                .sheets
                .into_iter()
                .map(|sheet| {
                    let chart_type = sheet
                        .chart_type
                        .parse::<ChartType>()
                        .map_err(|_| eyre::eyre!("parse chart type"))?;
                    let diff_category = sheet
                        .difficulty
                        .parse::<DifficultyCategory>()
                        .map_err(|_| eyre::eyre!("parse difficulty"))?;
                    let internal_level = sheet
                        .internal_level
                        .as_deref()
                        .and_then(|value| value.trim().parse::<f32>().ok());

                    Ok::<_, eyre::Error>(SongCatalogSheet {
                        chart_type,
                        diff_category,
                        level: sheet.level,
                        version: sheet.version_name,
                        internal_level,
                        region: sheet.region,
                    })
                })
                .collect::<Result<Vec<_>>>()?;

            Ok(SongCatalogSong {
                title: song.title,
                genre: song.genre.to_string(),
                artist: song.artist,
                image_name: song.image_name,
                aliases: song.aliases,
                sheets,
            })
        })
        .collect()
}

impl SongDatabaseClient {
    pub fn new(base_url: String) -> Result<Self> {
        let client = build_client()?;
        Ok(Self {
            client,
            base_url,
            cache: Arc::new(RwLock::new(None)),
        })
    }

    pub(crate) async fn list_song_catalog(&self) -> Result<Vec<SongCatalogSong>> {
        const SONG_DATABASE_CACHE_TTL: Duration = Duration::from_secs(600);

        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.as_ref()
                && cached.fetched_at.elapsed() < SONG_DATABASE_CACHE_TTL
            {
                return Ok(cached.songs.clone());
            }
        }

        let url = format!("{}/data.json", self.base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .wrap_err("fetch song database")?;

        if !resp.status().is_success() {
            return Err(eyre::eyre!(
                "Failed to fetch song database: HTTP {}",
                resp.status()
            ));
        }

        let response = resp
            .json::<models::SongDatabase>()
            .await
            .wrap_err("parse song database response")?;
        let songs = convert_song_catalog(response)?;

        let mut cache = self.cache.write().await;
        *cache = Some(CachedSongCatalog {
            songs: songs.clone(),
            fetched_at: Instant::now(),
        });

        Ok(songs)
    }

    pub(crate) fn cover_url(&self, image_name: &str) -> String {
        format!(
            "{}/cover/{}",
            self.base_url,
            urlencoding::encode(image_name)
        )
    }

    pub(crate) async fn search_song_metadata(
        &self,
        request: &SongMetadataSearchRequest,
    ) -> Result<SongMetadataSearchResponse> {
        let songs = self.list_song_catalog().await?;
        let mut items = Vec::new();

        for song in songs {
            let matches_song = request
                .title
                .as_deref()
                .is_none_or(|title| song.title == title)
                && request
                    .genre
                    .as_deref()
                    .is_none_or(|genre| song.genre == genre)
                && request
                    .artist
                    .as_deref()
                    .is_none_or(|artist| song.artist == artist);

            if !matches_song {
                continue;
            }

            for sheet in song.sheets {
                if request
                    .chart_type
                    .as_deref()
                    .is_some_and(|chart_type| sheet.chart_type.as_str() != chart_type)
                {
                    continue;
                }
                if request
                    .diff_category
                    .as_deref()
                    .is_some_and(|diff_category| sheet.diff_category.as_str() != diff_category)
                {
                    continue;
                }

                items.push(SongMetadata {
                    title: song.title.clone(),
                    chart_type: sheet.chart_type,
                    diff_category: sheet.diff_category,
                    level: Some(sheet.level.clone()),
                    internal_level: sheet.internal_level,
                    image_name: song.image_name.clone(),
                    version: sheet.version.clone(),
                    genre: song.genre.clone(),
                    artist: song.artist.clone(),
                    aliases: song.aliases.clone(),
                    region: sheet.region.clone(),
                });
            }
        }

        let total = items.len();

        if let Some(limit) = request.limits {
            items.truncate(limit);
        }

        Ok(SongMetadataSearchResponse { total, items })
    }

    pub(crate) async fn find_song_metadata(
        &self,
        title: &str,
        genre: &str,
        artist: &str,
        chart_type: ChartType,
        diff_category: DifficultyCategory,
    ) -> Result<Option<SongMetadata>> {
        let response = self
            .search_song_metadata(&SongMetadataSearchRequest {
                title: Some(title.to_string()),
                genre: Some(genre.to_string()),
                artist: Some(artist.to_string()),
                chart_type: Some(chart_type.as_str().to_string()),
                diff_category: Some(diff_category.as_str().to_string()),
                limits: Some(2),
            })
            .await?;

        if response.total > 1 {
            return Err(eyre::eyre!(
                "song metadata search returned multiple rows for exact identity: {title} / {genre} / {artist} [{chart_type} {diff_category}]"
            ));
        }

        Ok(response.items.into_iter().next())
    }
}

impl RecordCollectorClient {
    pub fn new(base_url: String) -> Result<Self> {
        let client = build_client()?;
        Ok(Self { client, base_url })
    }

    pub(crate) fn base_url(&self) -> &str {
        &self.base_url
    }

    pub(crate) async fn health_check(&self) -> Result<()> {
        let url = format!("{}/health/ready", self.base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .wrap_err("check record collector readiness")?;

        if resp.status().is_success() {
            return Ok(());
        }

        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if let Ok(parsed) = serde_json::from_str::<RecordCollectorErrorResponse>(&body) {
            return Err(ApiError::from_record_collector(status, parsed).into());
        }

        Err(ApiError::from_http_text(status, &body).into())
    }

    pub(crate) async fn get_player_profile(&self) -> Result<ParsedPlayerProfile> {
        self.get_with_retry("/api/player").await
    }

    pub(crate) async fn get_version_status(&self) -> RecordCollectorVersionStatus {
        if let Ok(mut cache) = version_status_cache().lock() {
            cache.retain(|_, cached| cached.fetched_at.elapsed() <= VERSION_STATUS_CACHE_TTL);
            if let Some(cached) = cache.get(&self.base_url).cloned() {
                return cached.status;
            }
        }

        let status = match self
            .get_with_retry::<VersionApiResponse>("/api/version")
            .await
        {
            Ok(response) => match is_minor_or_more_outdated(BOT_VERSION, &response.version) {
                Ok(true) => RecordCollectorVersionStatus::outdated(
                    Some(response.version),
                    RecordCollectorVersionIssue::VersionMismatch,
                ),
                Ok(false) => RecordCollectorVersionStatus::compatible(response.version),
                Err(err) => {
                    tracing::warn!(
                        "record collector {} returned invalid version {:?}: {err:#}",
                        self.base_url,
                        response.version
                    );
                    RecordCollectorVersionStatus::outdated(
                        Some(response.version),
                        RecordCollectorVersionIssue::InvalidResponse,
                    )
                }
            },
            Err(err) => {
                tracing::warn!(
                    "failed to load record collector version from {}: {err:#}",
                    self.base_url
                );
                RecordCollectorVersionStatus::outdated(
                    None,
                    RecordCollectorVersionIssue::Unreachable,
                )
            }
        };

        if let Ok(mut cache) = version_status_cache().lock() {
            cache.insert(
                self.base_url.clone(),
                CachedRecordCollectorVersionStatus {
                    status: status.clone(),
                    fetched_at: Instant::now(),
                },
            );
        }

        status
    }

    pub async fn get_recent(&self, limit: usize) -> Result<Vec<PlayRecordApiResponse>> {
        self.get_with_retry(&format!("/api/recent?limit={}", limit))
            .await
    }

    pub async fn get_today(&self, day: &str) -> Result<Vec<PlayRecordApiResponse>> {
        self.get_with_retry(&format!("/api/today?day={}", day))
            .await
    }

    pub async fn get_song_detail_scores(
        &self,
        title: &str,
        genre: &str,
        artist: &str,
    ) -> Result<Vec<SongDetailScoreApiResponse>> {
        self.get_with_retry(&format!(
            "/api/songs/scores?title={}&genre={}&artist={}",
            urlencoding::encode(title),
            urlencoding::encode(genre),
            urlencoding::encode(artist)
        ))
        .await
    }

    async fn get_with_retry<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        for attempt in 0..3 {
            match self.client.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    return resp.json().await.wrap_err("deserialize response");
                }
                Ok(resp) => {
                    let status = resp.status();
                    if attempt < 2 {
                        sleep(Duration::from_millis(100 * 2_u64.pow(attempt))).await;
                        continue;
                    }
                    let body = resp.text().await.unwrap_or_default();
                    if let Ok(parsed) = serde_json::from_str::<RecordCollectorErrorResponse>(&body)
                    {
                        return Err(ApiError::from_record_collector(status, parsed).into());
                    }
                    return Err(ApiError::from_http_text(status, &body).into());
                }
                Err(_e) if attempt < 2 => {
                    sleep(Duration::from_millis(100 * 2_u64.pow(attempt))).await;
                    continue;
                }
                Err(e) => return Err(e.into()),
            }
        }
        unreachable!()
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_record_collector_url;

    #[test]
    fn normalize_record_collector_url_rejects_invalid_input() {
        assert!(normalize_record_collector_url("").is_err());
        assert!(normalize_record_collector_url("ftp://example.com").is_err());
        assert!(normalize_record_collector_url("not a url").is_err());
    }

    #[test]
    fn normalize_record_collector_url_keeps_origin_only() {
        let normalized = normalize_record_collector_url(
            " https://collector.example:3000/api/player?foo=bar#frag ",
        )
        .expect("url should normalize");

        assert_eq!(normalized, "https://collector.example:3000");
    }

    #[test]
    fn normalize_record_collector_url_preserves_path_prefix() {
        let normalized = normalize_record_collector_url(
            " https://collector.example:3000/maistats?foo=bar#frag ",
        )
        .expect("url should normalize");

        assert_eq!(normalized, "https://collector.example:3000/maistats");
    }

    #[test]
    fn normalize_record_collector_url_strips_player_endpoint_but_keeps_prefix() {
        let normalized = normalize_record_collector_url(
            " https://collector.example:3000/maistats/api/player?foo=bar#frag ",
        )
        .expect("url should normalize");

        assert_eq!(normalized, "https://collector.example:3000/maistats");
    }

    #[test]
    fn normalize_record_collector_url_strips_version_endpoint_but_keeps_prefix() {
        let normalized = normalize_record_collector_url(
            " https://collector.example:3000/maistats/api/version?foo=bar#frag ",
        )
        .expect("url should normalize");

        assert_eq!(normalized, "https://collector.example:3000/maistats");
    }

    #[test]
    fn normalize_record_collector_url_strips_health_endpoint_but_keeps_prefix() {
        let normalized = normalize_record_collector_url(
            " https://collector.example:3000/maistats/health/ready?foo=bar#frag ",
        )
        .expect("url should normalize");

        assert_eq!(normalized, "https://collector.example:3000/maistats");
    }
}
