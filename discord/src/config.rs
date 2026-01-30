use eyre::WrapErr;

#[derive(Debug, Clone)]
pub struct DiscordConfig {
    pub bot_token: String,
    pub user_id: String,
    pub backend_url: String,
}

impl DiscordConfig {
    pub fn from_env() -> eyre::Result<Self> {
        let bot_token =
            std::env::var("DISCORD_BOT_TOKEN").wrap_err("missing env var: DISCORD_BOT_TOKEN")?;
        let user_id =
            std::env::var("DISCORD_USER_ID").wrap_err("missing env var: DISCORD_USER_ID")?;
        let backend_url = std::env::var("BACKEND_URL").wrap_err("missing env var: BACKEND_URL")?;

        Ok(Self {
            bot_token,
            user_id,
            backend_url,
        })
    }
}
