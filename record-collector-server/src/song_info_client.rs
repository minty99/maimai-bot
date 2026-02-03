use crate::error::{AppError, Result};
use reqwest::Client;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct SongMetadata {
    pub internal_level: Option<f32>,
    pub image_name: Option<String>,
    pub version: Option<String>,
    pub bucket: Option<String>,
}

impl SongMetadata {
    pub fn empty() -> Self {
        Self {
            internal_level: None,
            image_name: None,
            version: None,
            bucket: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SongInfoClient {
    pub base_url: String,
    pub client: Client,
}

impl SongInfoClient {
    pub fn new(base_url: String, client: Client) -> Self {
        let base_url = base_url.trim_end_matches('/').to_string();
        Self { base_url, client }
    }

    pub async fn get_song_metadata(
        &self,
        title: &str,
        chart_type: &str,
        diff_category: &str,
    ) -> Result<SongMetadata> {
        let url = format!(
            "{}/api/songs/{}/{}/{}",
            self.base_url,
            urlencoding::encode(title),
            urlencoding::encode(chart_type),
            urlencoding::encode(diff_category)
        );

        let resp = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| AppError::HttpClientError(format!("song info request failed: {e}")))?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(SongMetadata::empty());
        }

        if !resp.status().is_success() {
            return Err(AppError::HttpClientError(format!(
                "song info server error: HTTP {}",
                resp.status()
            )));
        }

        resp.json::<SongMetadata>()
            .await
            .map_err(|e| AppError::HttpClientError(format!("song info parse failed: {e}")))
    }
}
