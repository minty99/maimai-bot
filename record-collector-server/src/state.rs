use crate::config::RecordCollectorConfig;
use maimai_http_client::MaimaiClient;
use reqwest::Client;
use sqlx::SqlitePool;
use std::path::PathBuf;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: SqlitePool,
    pub config: RecordCollectorConfig,
    pub http_client: Client,
}

impl AppState {
    pub fn maimai_client(&self) -> eyre::Result<MaimaiClient> {
        let data_dir = PathBuf::from(&self.config.data_dir);
        let cookie_path = data_dir.join("cookies.json");

        let app_config = models::config::AppConfig {
            sega_id: self.config.sega_id.clone(),
            sega_password: self.config.sega_password.clone(),
            data_dir,
            cookie_path,
            discord_bot_token: None,
            discord_user_id: None,
        };
        MaimaiClient::new(&app_config)
    }
}
