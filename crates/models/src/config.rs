use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub sega_id: String,
    pub sega_password: String,
    pub data_dir: PathBuf,
    pub cookie_path: PathBuf,
    pub discord_bot_token: Option<String>,
    pub discord_user_id: Option<String>,
}
