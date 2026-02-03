use crate::config::BackendConfig;
use maimai_http_client::MaimaiClient;
use models::SongDataIndex;
use sqlx::SqlitePool;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct AppState {
    pub db_pool: SqlitePool,
    pub config: BackendConfig,
    pub song_data: Arc<RwLock<Arc<SongDataIndex>>>,
    pub song_data_base_path: PathBuf,
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

    pub fn reload_song_data(&self) -> eyre::Result<()> {
        let new_data = models::SongDataIndex::load_with_base_path(
            &self.song_data_base_path.to_string_lossy(),
        )?
        .unwrap_or_else(SongDataIndex::empty);

        let mut song_data = self.song_data.write().unwrap();
        *song_data = Arc::new(new_data);

        Ok(())
    }
}
