use eyre::{Result, WrapErr};
use models::{
    ChartType, DifficultyCategory, ParsedPlayerProfile, PlayRecordApiResponse, ScoreApiResponse,
    SongAliases, SongChartRegion, SongDetailScoreApiResponse, VersionApiResponse,
};
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;
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
pub struct ApiError {
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

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn message(&self) -> &str {
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
pub struct SongMetadata {
    pub title: String,
    pub chart_type: ChartType,
    pub diff_category: DifficultyCategory,
    pub level: Option<String>,
    pub internal_level: Option<f32>,
    pub image_name: Option<String>,
    pub version: Option<String>,
    pub genre: String,
    pub artist: String,
    pub aliases: SongAliases,
    pub region: SongChartRegion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongCatalogSheet {
    pub chart_type: ChartType,
    #[serde(rename = "difficulty")]
    pub diff_category: DifficultyCategory,
    pub level: String,
    pub version: Option<String>,
    pub internal_level: Option<f32>,
    pub region: SongChartRegion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongCatalogSong {
    pub title: String,
    pub genre: String,
    pub artist: String,
    pub image_name: Option<String>,
    pub aliases: SongAliases,
    pub sheets: Vec<SongCatalogSheet>,
}

#[derive(Debug, Clone)]
struct CachedSongCatalog {
    songs: Vec<SongCatalogSong>,
    fetched_at: Instant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongMetadataSearchRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub genre: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chart_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limits: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongMetadataSearchResponse {
    pub total: usize,
    pub items: Vec<SongMetadata>,
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

fn build_client() -> Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .wrap_err("build http client")
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

pub fn normalize_record_collector_url(input: &str) -> Result<String> {
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

    pub async fn list_song_catalog(&self) -> Result<Vec<SongCatalogSong>> {
        const SONG_DATABASE_CACHE_TTL: Duration = Duration::from_secs(3600);

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

    pub fn cover_url(&self, image_name: &str) -> String {
        format!(
            "{}/cover/{}",
            self.base_url,
            urlencoding::encode(image_name)
        )
    }

    pub async fn search_song_metadata(
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

    pub async fn find_song_metadata(
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

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub async fn trigger_poll(&self) {
        let url = format!("{}/api/poll", self.base_url);
        match self.client.post(&url).send().await {
            Ok(resp) if resp.status().is_success() || resp.status().as_u16() == 202 => {}
            Ok(resp) => {
                tracing::warn!(
                    "trigger_poll returned unexpected status {} from {}",
                    resp.status(),
                    self.base_url
                );
            }
            Err(err) => {
                tracing::warn!("trigger_poll failed for {}: {err:#}", self.base_url);
            }
        }
    }

    pub async fn health_check(&self) -> Result<()> {
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

    pub async fn get_player_profile(&self) -> Result<ParsedPlayerProfile> {
        self.get_with_retry("/api/player").await
    }

    pub async fn get_version(&self) -> Result<String> {
        let response: VersionApiResponse = self.get_with_retry("/api/version").await?;
        Ok(response.version)
    }

    pub async fn get_recent(&self, limit: usize) -> Result<Vec<PlayRecordApiResponse>> {
        self.get_with_retry(&format!("/api/recent?limit={}", limit))
            .await
    }

    pub async fn get_today(&self, day: &str) -> Result<Vec<PlayRecordApiResponse>> {
        self.get_with_retry(&format!("/api/today?day={}", day))
            .await
    }

    pub async fn get_all_rated_scores(&self) -> Result<Vec<ScoreApiResponse>> {
        self.get_with_retry("/api/scores/rated").await
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
