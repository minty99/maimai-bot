use crate::config::RecordCollectorConfig;
use crate::http_client::MaimaiClient;
use sqlx::SqlitePool;
use std::path::PathBuf;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: SqlitePool,
    pub config: RecordCollectorConfig,
}

impl AppState {
    pub(crate) fn maimai_client(&self) -> eyre::Result<MaimaiClient> {
        let data_dir = PathBuf::from(&self.config.data_dir);
        let cookie_path =
            std::env::temp_dir().join(format!("maistats-cookies-{}.json", std::process::id()));

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
