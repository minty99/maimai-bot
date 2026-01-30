use eyre::{Result, WrapErr};
use reqwest::Url;
use sqlx::SqlitePool;
use tracing::info;

use maimai_db::{clear_scores, get_app_state_u32, set_app_state_u32, upsert_playlogs, upsert_scores};
use maimai_http_client::{is_maintenance_window_now, MaimaiClient};
use maimai_parsers::{parse_player_data_html, parse_recent_html, parse_scores_html};
use models::{config::AppConfig, ParsedPlayRecord, ParsedPlayerData};

use crate::config::BackendConfig;

const STATE_KEY_TOTAL_PLAY_COUNT: &str = "player.total_play_count";
const STATE_KEY_RATING: &str = "player.rating";

pub async fn startup_sync(db_pool: &SqlitePool, config: &BackendConfig) -> Result<()> {
    info!("Starting startup sync...");

    if is_maintenance_window_now() {
        info!("Skipping startup sync due to maintenance window (04:00-07:00 local time)");
        return Ok(());
    }

    let app_config = backend_config_to_app_config(config);
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

        let entries = annotate_recent_entries_with_play_count(entries, player_data.total_play_count);
        let scraped_at = unix_timestamp();
        let count_total = entries.len();
        let count_with_idx = entries
            .iter()
            .filter(|e| e.played_at_unixtime.is_some())
            .count();

        upsert_playlogs(db_pool, scraped_at, &entries)
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

fn backend_config_to_app_config(config: &BackendConfig) -> AppConfig {
    use std::path::PathBuf;
    
    AppConfig {
        sega_id: config.sega_id.clone(),
        sega_password: config.sega_password.clone(),
        data_dir: PathBuf::from("data"),
        cookie_path: PathBuf::from("data/cookies.json"),
        discord_bot_token: None,
        discord_user_id: None,
    }
}

async fn fetch_player_data_logged_in(client: &MaimaiClient) -> Result<ParsedPlayerData> {
    let url = Url::parse("https://maimaidx-eng.com/maimai-mobile/playerData/")
        .wrap_err("parse playerData url")?;
    let bytes = client
        .get_bytes(&url)
        .await
        .wrap_err("fetch playerData url")?;
    let html = String::from_utf8(bytes).wrap_err("playerData response is not utf-8")?;
    parse_player_data_html(&html).wrap_err("parse playerData html")
}

async fn fetch_recent_entries_logged_in(client: &MaimaiClient) -> Result<Vec<ParsedPlayRecord>> {
    let url = Url::parse("https://maimaidx-eng.com/maimai-mobile/record/")
        .wrap_err("parse record url")?;
    let bytes = client.get_bytes(&url).await.wrap_err("fetch record url")?;
    let html = String::from_utf8(bytes).wrap_err("record response is not utf-8")?;
    parse_recent_html(&html).wrap_err("parse recent html")
}

async fn rebuild_scores_with_client(pool: &SqlitePool, client: &MaimaiClient) -> Result<usize> {
    clear_scores(pool).await.wrap_err("clear scores")?;

    let scraped_at = unix_timestamp();
    let mut all = Vec::new();

    for diff in 0u8..=4 {
        let url = scores_url(diff).wrap_err("build scores url")?;
        let bytes = client.get_bytes(&url).await.wrap_err("fetch scores url")?;
        let html = String::from_utf8(bytes).wrap_err("scores response is not utf-8")?;
        let mut entries = parse_scores_html(&html, diff).wrap_err("parse scores html")?;
        all.append(&mut entries);
    }

    let count = all.len();
    upsert_scores(pool, scraped_at, &all)
        .await
        .wrap_err("upsert scores")?;

    Ok(count)
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

async fn persist_player_snapshot(pool: &SqlitePool, player_data: &ParsedPlayerData) -> Result<()> {
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

fn scores_url(diff: u8) -> Result<Url> {
    if diff > 4 {
        return Err(eyre::eyre!("diff must be 0..4"));
    }
    Url::parse(&format!(
        "https://maimaidx-eng.com/maimai-mobile/record/musicGenre/search/?genre=99&diff={diff}"
    ))
    .wrap_err("parse scores url")
}

fn unix_timestamp() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
