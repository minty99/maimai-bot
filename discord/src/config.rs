use eyre::WrapErr;

#[derive(Debug, Clone)]
pub struct DiscordConfig {
    pub bot_token: String,
    pub user_id: String,
    pub song_info_server_url: String,
    pub record_collector_server_url: String,
}

impl DiscordConfig {
    pub fn from_env() -> eyre::Result<Self> {
        let bot_token =
            std::env::var("DISCORD_BOT_TOKEN").wrap_err("missing env var: DISCORD_BOT_TOKEN")?;
        let user_id =
            std::env::var("DISCORD_USER_ID").wrap_err("missing env var: DISCORD_USER_ID")?;
        let song_info_server_url = std::env::var("SONG_INFO_SERVER_URL")
            .unwrap_or_else(|_| "http://localhost:3001".to_string());
        let record_collector_server_url = std::env::var("RECORD_COLLECTOR_SERVER_URL")
            .unwrap_or_else(|_| "http://localhost:3000".to_string());

        Ok(Self {
            bot_token,
            user_id,
            song_info_server_url,
            record_collector_server_url,
        })
    }
}
