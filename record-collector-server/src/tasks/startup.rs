use eyre::{Result, WrapErr};
use sqlx::SqlitePool;
use tracing::info;

use crate::db::{get_app_state_u32, set_app_state_u32, upsert_playlogs};
use crate::http_client::{MaimaiClient, is_maintenance_window_now};
use maimai_parsers::{parse_player_data_html, parse_recent_html};
use models::{ParsedPlayRecord, ParsedPlayerProfile, config::AppConfig};

use crate::config::RecordCollectorConfig;
use crate::tasks::scores_sync::rebuild_scores_with_client;

const STATE_KEY_TOTAL_PLAY_COUNT: &str = "player.total_play_count";
const STATE_KEY_RATING: &str = "player.rating";

pub(crate) async fn startup_sync(
    db_pool: &SqlitePool,
    config: &RecordCollectorConfig,
) -> Result<()> {
    info!("Starting startup sync...");

    if is_maintenance_window_now() {
        info!("Skipping startup sync due to maintenance window (04:00-07:00 local time)");
        return Ok(());
    }

    let app_config = to_app_config(config);
    let mut client = MaimaiClient::new(&app_config).wrap_err("create HTTP client")?;
    client
        .ensure_logged_in()
        .await
        .wrap_err("ensure logged in")?;

    let player_data = fetch_player_data_logged_in(&client)
        .await
        .wrap_err("fetch player data")?;

    info!(
        "Player data fetched: user_name={}, total_play_count={}, rating={}",
        player_data.user_name, player_data.total_play_count, player_data.rating
    );

    let stored_total = get_app_state_u32(db_pool, STATE_KEY_TOTAL_PLAY_COUNT)
        .await
        .ok()
        .flatten();

    let should_sync = match stored_total {
        Some(v) if v == player_data.total_play_count => {
            info!("Play count unchanged ({}); skipping sync", v);
            false
        }
        Some(v) => {
            info!(
                "Play count changed: {} -> {}; will sync",
                v, player_data.total_play_count
            );
            true
        }
        None => {
            info!("No stored play count; will perform initial sync");
            true
        }
    };

    if should_sync {
        let scores_count = rebuild_scores_with_client(db_pool, &client)
            .await
            .wrap_err("rebuild scores")?;
        info!("Scores synced: entries={}", scores_count);

        let entries = fetch_recent_entries_logged_in(&client)
            .await
            .wrap_err("fetch recent entries")?;

        let entries =
            annotate_recent_entries_with_play_count(entries, player_data.total_play_count);
        let count_total = entries.len();
        let count_with_idx = entries
            .iter()
            .filter(|e| e.played_at_unixtime.is_some())
            .count();

        upsert_playlogs(db_pool, &entries)
            .await
            .wrap_err("upsert playlogs")?;

        info!(
            "Recent playlogs synced: entries_total={} entries_with_idx={}",
            count_total, count_with_idx
        );
    }

    persist_player_snapshot(db_pool, &player_data)
        .await
        .wrap_err("persist player snapshot")?;

    info!("Startup sync complete");
    Ok(())
}

fn to_app_config(config: &RecordCollectorConfig) -> AppConfig {
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

async fn fetch_player_data_logged_in(client: &MaimaiClient) -> Result<ParsedPlayerProfile> {
    let url = reqwest::Url::parse("https://maimaidx-eng.com/maimai-mobile/playerData/")
        .wrap_err("parse playerData url")?;
    let bytes = client
        .get_bytes(&url)
        .await
        .wrap_err("fetch playerData url")?;
    let html = String::from_utf8(bytes).wrap_err("playerData response is not utf-8")?;
    parse_player_data_html(&html).wrap_err("parse playerData html")
}

async fn fetch_recent_entries_logged_in(client: &MaimaiClient) -> Result<Vec<ParsedPlayRecord>> {
    let url = reqwest::Url::parse("https://maimaidx-eng.com/maimai-mobile/record/")
        .wrap_err("parse record url")?;
    let bytes = client.get_bytes(&url).await.wrap_err("fetch record url")?;
    let html = String::from_utf8(bytes).wrap_err("record response is not utf-8")?;
    parse_recent_html(&html).wrap_err("parse recent html")
}

fn annotate_recent_entries_with_play_count(
    mut entries: Vec<ParsedPlayRecord>,
    total_play_count: u32,
) -> Vec<ParsedPlayRecord> {
    let Some(last_track_01_idx) = entries.iter().rposition(|e| e.track == Some(1)) else {
        return Vec::new();
    };
    entries.truncate(last_track_01_idx + 1);

    let mut credit_idx: u32 = 0;
    for entry in &mut entries {
        entry.credit_play_count = Some(total_play_count.saturating_sub(credit_idx));

        if entry.track == Some(1) {
            credit_idx = credit_idx.saturating_add(1);
        }
    }

    entries
}

async fn persist_player_snapshot(
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
