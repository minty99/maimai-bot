use std::sync::Arc;

use poise::serenity_prelude as serenity;
use tokio::sync::RwLock;

use crate::config::AppConfig;
use crate::db::SqlitePool;
use crate::http::MaimaiClient;
use crate::song_data::SongDataIndex;

pub(crate) type Error = eyre::Report;
pub(crate) type Context<'a> = poise::Context<'a, BotData, Error>;

#[derive(Debug, Clone)]
pub struct BotData {
    pub db: SqlitePool,
    pub maimai_client: Arc<MaimaiClient>,
    pub config: AppConfig,
    pub discord_user_id: serenity::UserId,
    pub discord_http: Arc<serenity::Http>,
    pub maimai_user_name: Arc<RwLock<String>>,
    pub song_data: Option<Arc<SongDataIndex>>,
}
