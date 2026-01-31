use crate::config::BackendConfig;
use maimai_http_client::MaimaiClient;
use models::SongDataIndex;
use sqlx::SqlitePool;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: SqlitePool,
    pub config: BackendConfig,
    pub song_data: Option<Arc<SongDataIndex>>,
    pub fetched_data_path: String,
}

impl AppState {
    pub fn maimai_client(&self) -> eyre::Result<MaimaiClient> {
        let app_config = models::config::AppConfig {
            sega_id: self.config.sega_id.clone(),
            sega_password: self.config.sega_password.clone(),
            data_dir: PathBuf::from("data"),
            cookie_path: PathBuf::from("data/cookies.json"),
            discord_bot_token: None,
            discord_user_id: None,
        };
        MaimaiClient::new(&app_config)
    }
}
