use eyre::{Result, WrapErr};
use models::{
    ChartType, DifficultyCategory, ParsedPlayerProfile, ParsedRatingTargets, PlayRecordApiResponse,
    ScoreApiResponse, SongChartRegion, SongDetailScoreApiResponse,
};
use reqwest::Client;
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

pub(crate) enum PlayerDataResult {
    Ok(ParsedPlayerProfile),
    Maintenance,
    Unavailable(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SongMetadata {
    pub(crate) level: Option<String>,
    pub(crate) internal_level: Option<f32>,
    pub(crate) user_level: Option<String>,
    pub(crate) image_name: Option<String>,
    pub(crate) version: Option<String>,
    pub(crate) genre: String,
    pub(crate) artist: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SongInfoSheet {
    pub(crate) chart_type: ChartType,
    pub(crate) difficulty: DifficultyCategory,
    pub(crate) level: String,
    pub(crate) version: Option<String>,
    pub(crate) internal_level: Option<f32>,
    pub(crate) user_level: Option<String>,
    pub(crate) region: SongChartRegion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SongInfo {
    pub(crate) title: String,
    pub(crate) genre: String,
    pub(crate) artist: String,
    pub(crate) image_name: Option<String>,
    pub(crate) sheets: Vec<SongInfoSheet>,
}

#[derive(Debug)]
pub struct SongInfoClient {
    client: Client,
    base_url: String,
}

#[derive(Debug)]
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

    pub(crate) async fn get_song_metadata(
        &self,
        title: &str,
        chart_type: &str,
        diff_category: &str,
    ) -> Result<Option<SongMetadata>> {
        let url = format!(
            "{}/api/songs/{}/{}/{}",
            self.base_url,
            urlencoding::encode(title),
            urlencoding::encode(chart_type),
            urlencoding::encode(diff_category)
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .wrap_err("fetch song metadata")?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !resp.status().is_success() {
            return Err(eyre::eyre!(
                "Failed to fetch song metadata: HTTP {}",
                resp.status()
            ));
        }

        let metadata = resp
            .json::<SongMetadata>()
            .await
            .wrap_err("parse song metadata")?;
        Ok(Some(metadata))
    }

    pub(crate) async fn get_song_metadata_by_identity(
        &self,
        title: &str,
        genre: &str,
        artist: &str,
        chart_type: &str,
        diff_category: &str,
    ) -> Result<Option<SongMetadata>> {
        let url = format!("{}/api/songs/metadata", self.base_url);

        let resp = self
            .client
            .get(&url)
            .query(&[
                ("title", title),
                ("genre", genre),
                ("artist", artist),
                ("chart_type", chart_type),
                ("diff_category", diff_category),
            ])
            .send()
            .await
            .wrap_err("fetch song metadata by identity")?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !resp.status().is_success() {
            return Err(eyre::eyre!(
                "Failed to fetch song metadata by identity: HTTP {}",
                resp.status()
            ));
        }

        let metadata = resp
            .json::<SongMetadata>()
            .await
            .wrap_err("parse song metadata by identity")?;
        Ok(Some(metadata))
    }

    pub(crate) async fn get_song_info_by_title(&self, title: &str) -> Result<Option<SongInfo>> {
        let url = format!(
            "{}/api/songs/by-title/{}",
            self.base_url,
            urlencoding::encode(title)
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .wrap_err("fetch song info")?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !resp.status().is_success() {
            return Err(eyre::eyre!(
                "Failed to fetch song info: HTTP {}",
                resp.status()
            ));
        }

        let song_info = resp.json::<SongInfo>().await.wrap_err("parse song info")?;
        Ok(Some(song_info))
    }
}

impl RecordCollectorClient {
    pub fn new(base_url: String) -> Result<Self> {
        let client = build_client()?;
        Ok(Self { client, base_url })
    }

    pub(crate) async fn get_player(&self) -> PlayerDataResult {
        let url = format!("{}/api/player", self.base_url);
        for attempt in 0..3 {
            match self.client.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    match resp.json::<ParsedPlayerProfile>().await {
                        Ok(data) => return PlayerDataResult::Ok(data),
                        Err(e) => {
                            if attempt < 2 {
                                sleep(Duration::from_millis(100 * 2_u64.pow(attempt))).await;
                                continue;
                            }
                            return PlayerDataResult::Unavailable(format!(
                                "Failed to parse response: {}",
                                e
                            ));
                        }
                    }
                }
                Ok(resp) => {
                    let status = resp.status();
                    if let Ok(error_body) = resp.json::<RecordCollectorErrorResponse>().await {
                        if error_body.maintenance == Some(true) {
                            return PlayerDataResult::Maintenance;
                        }
                        if attempt < 2 {
                            sleep(Duration::from_millis(100 * 2_u64.pow(attempt))).await;
                            continue;
                        }
                        return PlayerDataResult::Unavailable(format!(
                            "HTTP {}: {}",
                            status, error_body.message
                        ));
                    }
                    if attempt < 2 {
                        sleep(Duration::from_millis(100 * 2_u64.pow(attempt))).await;
                        continue;
                    }
                    return PlayerDataResult::Unavailable(format!("HTTP {}", status));
                }
                Err(e) => {
                    if attempt < 2 {
                        sleep(Duration::from_millis(100 * 2_u64.pow(attempt))).await;
                        continue;
                    }
                    return PlayerDataResult::Unavailable(format!("Connection error: {}", e));
                }
            }
        }
        PlayerDataResult::Unavailable("Max retries exceeded".to_string())
    }

    pub async fn search_scores(&self, query: &str) -> Result<Vec<ScoreApiResponse>> {
        self.get_with_retry(&format!(
            "/api/scores/search?q={}",
            urlencoding::encode(query)
        ))
        .await
    }

    pub async fn get_score(
        &self,
        title: &str,
        genre: &str,
        artist: &str,
        chart: &str,
        diff: &str,
    ) -> Result<ScoreApiResponse> {
        self.get_with_retry(&format!(
            "/api/scores/item?title={}&genre={}&artist={}&chart_type={}&diff_category={}",
            urlencoding::encode(title),
            urlencoding::encode(genre),
            urlencoding::encode(artist),
            urlencoding::encode(chart),
            urlencoding::encode(diff)
        ))
        .await
    }

    pub async fn get_recent(&self, limit: usize) -> Result<Vec<PlayRecordApiResponse>> {
        self.get_with_retry(&format!("/api/recent?limit={}", limit))
            .await
    }

    pub async fn get_today(&self, day: &str) -> Result<Vec<PlayRecordApiResponse>> {
        self.get_with_retry(&format!("/api/today?day={}", day))
            .await
    }

    pub async fn get_rated_scores(&self) -> Result<Vec<ScoreApiResponse>> {
        self.get_with_retry("/api/scores/rated").await
    }

    pub async fn get_rating_targets(&self) -> Result<ParsedRatingTargets> {
        self.get_with_retry("/api/rating/targets").await
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

    pub async fn health_check_with_retry(&self) -> Result<()> {
        let url = format!("{}/health/ready", self.base_url);
        let mut attempt = 0;
        const MAX_RETRIES: u32 = 5;

        loop {
            match self.client.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    tracing::info!("Record collector is ready");
                    return Ok(());
                }
                Ok(resp) => {
                    let status = resp.status();
                    if attempt < MAX_RETRIES {
                        let wait_ms = 1000 * 2_u64.pow(attempt);
                        tracing::warn!(
                            "Record collector not ready (HTTP {}), retrying in {}ms (attempt {}/{})",
                            status,
                            wait_ms,
                            attempt + 1,
                            MAX_RETRIES
                        );
                        sleep(Duration::from_millis(wait_ms)).await;
                        attempt += 1;
                        continue;
                    }
                    return Err(eyre::eyre!(
                        "Record collector failed to become ready after {} retries (HTTP {})",
                        MAX_RETRIES,
                        status
                    ));
                }
                Err(e) => {
                    if attempt < MAX_RETRIES {
                        let wait_ms = 1000 * 2_u64.pow(attempt);
                        tracing::warn!(
                            "Record collector connection failed: {}, retrying in {}ms (attempt {}/{})",
                            e,
                            wait_ms,
                            attempt + 1,
                            MAX_RETRIES
                        );
                        sleep(Duration::from_millis(wait_ms)).await;
                        attempt += 1;
                        continue;
                    }
                    return Err(eyre::eyre!(
                        "Record collector failed to become ready after {} retries: {}",
                        MAX_RETRIES,
                        e
                    ));
                }
            }
        }
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
                Err(e) if attempt < 2 => {
                    sleep(Duration::from_millis(100 * 2_u64.pow(attempt))).await;
                    continue;
                }
                Err(e) => return Err(e.into()),
            }
        }
        unreachable!()
    }
}
