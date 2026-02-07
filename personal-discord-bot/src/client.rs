use eyre::{Result, WrapErr};
use models::{ParsedPlayerData, PlayRecordResponse, ScoreResponse};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, Duration};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordCollectorErrorResponse {
    pub message: String,
    pub code: String,
    #[serde(default)]
    pub maintenance: Option<bool>,
}

pub enum PlayerDataResult {
    Ok(ParsedPlayerData),
    Maintenance,
    Unavailable(String),
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

    pub async fn get_cover(&self, image_name: &str) -> Result<Vec<u8>> {
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
}

impl RecordCollectorClient {
    pub fn new(base_url: String) -> Result<Self> {
        let client = build_client()?;
        Ok(Self { client, base_url })
    }

    pub async fn get_player(&self) -> PlayerDataResult {
        let url = format!("{}/api/player", self.base_url);
        for attempt in 0..3 {
            match self.client.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    match resp.json::<ParsedPlayerData>().await {
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

    pub async fn search_scores(&self, query: &str) -> Result<Vec<ScoreResponse>> {
        self.get_with_retry(&format!(
            "/api/scores/search?q={}",
            urlencoding::encode(query)
        ))
        .await
    }

    pub async fn get_score(&self, title: &str, chart: &str, diff: &str) -> Result<ScoreResponse> {
        self.get_with_retry(&format!(
            "/api/scores/{}/{}/{}",
            urlencoding::encode(title),
            chart,
            diff
        ))
        .await
    }

    pub async fn get_recent(&self, limit: usize) -> Result<Vec<PlayRecordResponse>> {
        self.get_with_retry(&format!("/api/recent?limit={}", limit))
            .await
    }

    pub async fn get_today(&self, day: &str) -> Result<Vec<PlayRecordResponse>> {
        self.get_with_retry(&format!("/api/today?day={}", day))
            .await
    }

    pub async fn get_rated_scores(&self) -> Result<Vec<ScoreResponse>> {
        self.get_with_retry("/api/scores/rated").await
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
                    let body = resp.text().await.unwrap_or_default();
                    if attempt < 2 {
                        sleep(Duration::from_millis(100 * 2_u64.pow(attempt))).await;
                        continue;
                    }
                    return Err(eyre::eyre!("HTTP {}: {}", status, body));
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
