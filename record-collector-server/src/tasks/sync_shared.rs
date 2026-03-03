use eyre::{Result, WrapErr};
use sqlx::SqlitePool;

use crate::config::RecordCollectorConfig;
use crate::db::set_app_state_u32;
use crate::http_client::MaimaiClient;
use maimai_parsers::{parse_player_data_html, parse_recent_html};
use models::{ParsedPlayRecord, ParsedPlayerProfile, config::AppConfig};

pub(crate) const STATE_KEY_TOTAL_PLAY_COUNT: &str = "player.total_play_count";
pub(crate) const STATE_KEY_RATING: &str = "player.rating";

pub(crate) fn to_app_config(config: &RecordCollectorConfig) -> AppConfig {
    use std::path::PathBuf;

    let data_dir = PathBuf::from(&config.data_dir);
    let cookie_path = data_dir.join("cookies.json");

    AppConfig {
        sega_id: config.sega_id.clone(),
        sega_password: config.sega_password.clone(),
        data_dir,
        cookie_path,
        discord_bot_token: None,
        discord_user_id: None,
    }
}

pub(crate) async fn fetch_player_data_logged_in(
    client: &MaimaiClient,
) -> Result<ParsedPlayerProfile> {
    let url = reqwest::Url::parse("https://maimaidx-eng.com/maimai-mobile/playerData/")
        .wrap_err("parse playerData url")?;
    let bytes = client
        .get_bytes(&url)
        .await
        .wrap_err("fetch playerData url")?;
    let html = String::from_utf8(bytes).wrap_err("playerData response is not utf-8")?;
    parse_player_data_html(&html).wrap_err("parse playerData html")
}

pub(crate) async fn fetch_recent_entries_logged_in(
    client: &MaimaiClient,
) -> Result<Vec<ParsedPlayRecord>> {
    let url = reqwest::Url::parse("https://maimaidx-eng.com/maimai-mobile/record/")
        .wrap_err("parse record url")?;
    let bytes = client.get_bytes(&url).await.wrap_err("fetch record url")?;
    let html = String::from_utf8(bytes).wrap_err("record response is not utf-8")?;
    parse_recent_html(&html).wrap_err("parse recent html")
}

pub(crate) fn annotate_recent_entries_with_play_count(
    mut entries: Vec<ParsedPlayRecord>,
    total_play_count: u32,
) -> Vec<ParsedPlayRecord> {
    let Some(last_track_01_idx) = entries.iter().rposition(|e| e.track == Some(1)) else {
        return Vec::new();
    };
    entries.truncate(last_track_01_idx + 1);

    let mut credit_idx: u32 = 0;
    for entry in &mut entries {
        entry.credit_id = Some(total_play_count.saturating_sub(credit_idx));

        if entry.track == Some(1) {
            credit_idx = credit_idx.saturating_add(1);
        }
    }

    entries
}

pub(crate) async fn persist_player_snapshot(
    pool: &SqlitePool,
    player_data: &ParsedPlayerProfile,
) -> Result<()> {
    let now = unix_timestamp();
    set_app_state_u32(
        pool,
        STATE_KEY_TOTAL_PLAY_COUNT,
        player_data.total_play_count,
        now,
    )
    .await
    .wrap_err("store total play count")?;
    set_app_state_u32(pool, STATE_KEY_RATING, player_data.rating, now)
        .await
        .wrap_err("store rating")?;
    Ok(())
}

fn unix_timestamp() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
