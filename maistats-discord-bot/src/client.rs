use eyre::{Result, WrapErr};
use models::{
    ChartType, DifficultyCategory, ParsedPlayerProfile, PlayRecordApiResponse, SongAliases,
    SongChartRegion, SongDetailScoreApiResponse,
};
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use std::fmt;
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
pub struct SongInfoClient {
    client: Client,
    base_url: String,
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

    for suffix in ["/health/ready", "/api/player"] {
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

impl SongInfoClient {
    pub fn new(base_url: String) -> Result<Self> {
        let client = build_client()?;
        Ok(Self { client, base_url })
    }

    pub(crate) async fn get_cover(&self, image_name: &str) -> Result<Vec<u8>> {
        let url = format!("{}/api/cover/{}", self.base_url, image_name);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .wrap_err("fetch cover image")?;

        if !resp.status().is_success() {
            return Err(eyre::eyre!("Failed to fetch cover: HTTP {}", resp.status()));
        }

        resp.bytes()
            .await
            .map(|b| b.to_vec())
            .wrap_err("read cover image bytes")
    }

    pub(crate) async fn search_song_metadata(
        &self,
        request: &SongMetadataSearchRequest,
    ) -> Result<SongMetadataSearchResponse> {
        let url = format!("{}/api/songs/metadata", self.base_url);

        let resp = self
            .client
            .post(&url)
            .json(request)
            .send()
            .await
            .wrap_err("search song metadata")?;
        if !resp.status().is_success() {
            return Err(eyre::eyre!(
                "Failed to search song metadata: HTTP {}",
                resp.status()
            ));
        }

        resp.json::<SongMetadataSearchResponse>()
            .await
            .wrap_err("parse song metadata search response")
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
    fn normalize_record_collector_url_strips_health_endpoint_but_keeps_prefix() {
        let normalized = normalize_record_collector_url(
            " https://collector.example:3000/maistats/health/ready?foo=bar#frag ",
        )
        .expect("url should normalize");

        assert_eq!(normalized, "https://collector.example:3000/maistats");
    }
}
