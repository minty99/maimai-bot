use eyre::WrapErr;

#[derive(Debug, Clone)]
pub struct DiscordConfig {
    pub bot_token: String,
    pub dev_user_id: String,
    pub song_info_server_url: String,
    pub database_url: String,
    pub data_dir: String,
}

impl DiscordConfig {
    pub fn from_env() -> eyre::Result<Self> {
        let bot_token =
            std::env::var("DISCORD_BOT_TOKEN").wrap_err("missing env var: DISCORD_BOT_TOKEN")?;
        let dev_user_id = std::env::var("DISCORD_DEV_USER_ID")
            .wrap_err("missing env var: DISCORD_DEV_USER_ID")?;
        let song_info_server_url = std::env::var("SONG_INFO_SERVER_URL")
            .unwrap_or_else(|_| "http://localhost:3001".to_string());
        let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string());
        let database_url = std::env::var("DISCORD_BOT_DATABASE_URL")
            .unwrap_or_else(|_| format!("sqlite:{data_dir}/maistats-discord-bot.sqlite3"));

        Ok(Self {
            bot_token,
            dev_user_id,
            song_info_server_url,
            database_url,
            data_dir,
        })
    }
}
