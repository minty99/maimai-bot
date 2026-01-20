use std::path::{Path, PathBuf};

use eyre::WrapErr;

use crate::cli::RootArgs;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub sega_id: String,
    pub sega_password: String,
    pub data_dir: PathBuf,
    pub cookie_path: PathBuf,
    pub discord_bot_token: Option<String>,
    pub discord_user_id: Option<String>,
}

impl AppConfig {
    pub fn from_env_and_args(args: &RootArgs) -> eyre::Result<Self> {
        let sega_id = std::env::var("SEGA_ID").wrap_err("missing env var: SEGA_ID")?;
        let sega_password =
            std::env::var("SEGA_PASSWORD").wrap_err("missing env var: SEGA_PASSWORD")?;
        let discord_bot_token = std::env::var("DISCORD_BOT_TOKEN").ok();
        let discord_user_id = std::env::var("DISCORD_USER_ID").ok();

        Ok(Self {
            sega_id,
            sega_password,
            data_dir: args.data_dir.clone(),
            cookie_path: args.cookie_path.clone(),
            discord_bot_token,
            discord_user_id,
        })
    }

    pub fn ensure_dirs(&self) -> eyre::Result<()> {
        ensure_parent_dir(&self.cookie_path)?;
        std::fs::create_dir_all(&self.data_dir).wrap_err("create data_dir")?;
        Ok(())
    }
}

fn ensure_parent_dir(path: &Path) -> eyre::Result<()> {
    let Some(parent) = path.parent() else {
        return Err(eyre::eyre!("invalid path: {path:?}"));
    };
    if parent.as_os_str().is_empty() {
        return Ok(());
    }
    std::fs::create_dir_all(parent).wrap_err("create parent dir")?;
    Ok(())
}
