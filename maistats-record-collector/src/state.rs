use crate::config::RecordCollectorConfig;
use crate::http_client::MaimaiClient;
use crate::logging::LogBuffer;
use sqlx::SqlitePool;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) db_pool: SqlitePool,
    pub(crate) config: RecordCollectorConfig,
    pub(crate) log_buffer: Arc<LogBuffer>,
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
