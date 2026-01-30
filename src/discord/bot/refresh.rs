use std::sync::Arc;
use std::time::Duration;

use eyre::{Result, WrapErr};
use poise::serenity_prelude as serenity;
use reqwest::Url;
use tokio::time::interval;
use tracing::{debug, error, info};

use crate::db;
use crate::db::{SqlitePool, format_chart_type};
use crate::http::MaimaiClient;
use crate::http::is_maintenance_window_now;
use crate::maimai::models::{ParsedPlayRecord, ParsedPlayerData};
use crate::maimai::parse::player_data::parse_player_data_html;
use crate::maimai::parse::recent::parse_recent_html;
use crate::maimai::parse::score_list::parse_scores_html;

use super::types::BotData;

const STATE_KEY_TOTAL_PLAY_COUNT: &str = "player.total_play_count";
const STATE_KEY_RATING: &str = "player.rating";

pub(crate) fn start_background_tasks(bot_data: BotData, _cache: Arc<serenity::Cache>) {
    tokio::spawn(async move {
        let mut timer = interval(Duration::from_secs(600));
        timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        info!("Background task started: periodic playerData poll (every 10 minutes)");

        loop {
            timer.tick().await;

            info!("Running periodic playerData poll...");

            match refresh_from_network_if_needed(&bot_data).await {
                Ok(true) => info!("New plays detected; refreshed DB"),
                Ok(false) => {}
                Err(e) => error!("Periodic poll failed: {e:#}"),
            }
        }
    });
}

pub(crate) async fn refresh_from_network_if_needed(bot_data: &BotData) -> Result<bool> {
    if is_maintenance_window_now() {
        info!("Skipping periodic poll due to maintenance window (04:00-07:00 local time)");
        return Ok(false);
    }

    let mut client = MaimaiClient::new(&bot_data.config).wrap_err("create HTTP client")?;
    client
        .ensure_logged_in()
        .await
        .wrap_err("ensure logged in")?;

    let player_data = fetch_player_data_logged_in(&client)
        .await
        .wrap_err("fetch player data")?;
    *bot_data.maimai_user_name.write().await = player_data.user_name.clone();

    let stored_total = db::get_app_state_u32(&bot_data.db, STATE_KEY_TOTAL_PLAY_COUNT).await;

    let stored_total = match stored_total {
        Ok(v) => v,
        Err(e) => {
            debug!("Failed to read stored total play count; treating as missing: {e:#}");
            None
        }
    };

    if let Some(stored_total) = stored_total
        && stored_total == player_data.total_play_count
    {
        return Ok(false);
    }

    let entries = fetch_recent_entries_logged_in(&client)
        .await
        .wrap_err("fetch recent")?;

    let mut entries =
        annotate_recent_entries_with_play_count(entries, player_data.total_play_count);

    if stored_total.is_some() {
        annotate_first_play_flags(&bot_data.db, &mut entries)
            .await
            .wrap_err("classify first plays")?;
    }
    let scraped_at = unix_timestamp();

    db::upsert_playlogs(&bot_data.db, scraped_at, &entries)
        .await
        .wrap_err("upsert playlogs")?;

    rebuild_scores_with_client(&bot_data.db, &client)
        .await
        .wrap_err("rebuild scores")?;
    persist_player_snapshot(&bot_data.db, &player_data)
        .await
        .wrap_err("persist player snapshot")?;

    if stored_total.is_some() {
        Ok(true)
    } else {
        debug!("No stored total play count; seeded DB without sending DM");
        Ok(false)
    }
}

pub(crate) async fn sync_from_network_without_discord(
    pool: &SqlitePool,
    client: &mut MaimaiClient,
) -> Result<ParsedPlayerData> {
    client
        .ensure_logged_in()
        .await
        .wrap_err("ensure logged in")?;

    let player_data = fetch_player_data_logged_in(client)
        .await
        .wrap_err("fetch player data")?;

    let stored_total = db::get_app_state_u32(pool, STATE_KEY_TOTAL_PLAY_COUNT)
        .await
        .ok()
        .flatten();

    if stored_total.is_some() && stored_total == Some(player_data.total_play_count) {
        return Ok(player_data);
    }

    let entries = fetch_recent_entries_logged_in(client)
        .await
        .wrap_err("fetch recent")?;
    let mut entries =
        annotate_recent_entries_with_play_count(entries, player_data.total_play_count);

    if stored_total.is_some() {
        annotate_first_play_flags(pool, &mut entries)
            .await
            .wrap_err("classify first plays")?;
    }

    let scraped_at = unix_timestamp();
    db::upsert_playlogs(pool, scraped_at, &entries)
        .await
        .wrap_err("upsert playlogs")?;

    rebuild_scores_with_client(pool, client)
        .await
        .wrap_err("rebuild scores")?;
    persist_player_snapshot(pool, &player_data)
        .await
        .wrap_err("persist player snapshot")?;

    Ok(player_data)
}

pub(crate) async fn fetch_player_data(bot_data: &BotData) -> Result<ParsedPlayerData> {
    let mut client = MaimaiClient::new(&bot_data.config).wrap_err("create HTTP client")?;
    client
        .ensure_logged_in()
        .await
        .wrap_err("ensure logged in")?;

    fetch_player_data_logged_in(&client).await
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

pub(crate) async fn should_sync_scores(
    pool: &SqlitePool,
    player_data: &ParsedPlayerData,
) -> Result<bool> {
    match db::get_app_state_u32(pool, STATE_KEY_TOTAL_PLAY_COUNT).await {
        Ok(Some(v)) => Ok(v != player_data.total_play_count),
        Ok(None) => {
            debug!("No stored total play count; will rebuild DB");
            Ok(true)
        }
        Err(e) => {
            debug!("Failed to read total play count from DB; will rebuild DB: {e:#}");
            Ok(true)
        }
    }
}

pub(crate) async fn persist_play_counts(
    pool: &SqlitePool,
    player_data: &ParsedPlayerData,
) -> Result<()> {
    let now = unix_timestamp();
    db::set_app_state_u32(
        pool,
        STATE_KEY_TOTAL_PLAY_COUNT,
        player_data.total_play_count,
        now,
    )
    .await
    .wrap_err("store total play count")?;
    Ok(())
}

pub(crate) async fn persist_player_snapshot(
    pool: &SqlitePool,
    player_data: &ParsedPlayerData,
) -> Result<()> {
    let now = unix_timestamp();
    db::set_app_state_u32(
        pool,
        STATE_KEY_TOTAL_PLAY_COUNT,
        player_data.total_play_count,
        now,
    )
    .await
    .wrap_err("store total play count")?;
    db::set_app_state_u32(pool, STATE_KEY_RATING, player_data.rating, now)
        .await
        .wrap_err("store rating")?;
    Ok(())
}

pub(crate) async fn initial_scores_sync(bot_data: &BotData) -> Result<()> {
    info!("Running startup scores sync (diff 0..4)...");

    let mut client = MaimaiClient::new(&bot_data.config).wrap_err("create HTTP client")?;
    client
        .ensure_logged_in()
        .await
        .wrap_err("ensure logged in")?;

    let count = rebuild_scores_with_client(&bot_data.db, &client)
        .await
        .wrap_err("rebuild scores")?;

    info!("Startup scores sync completed: entries={count}");
    Ok(())
}

async fn rebuild_scores_with_client(pool: &SqlitePool, client: &MaimaiClient) -> Result<usize> {
    db::clear_scores(pool).await.wrap_err("clear scores")?;

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
    db::upsert_scores(pool, scraped_at, &all)
        .await
        .wrap_err("upsert scores")?;

    Ok(count)
}

pub(crate) async fn initial_recent_sync(bot_data: &BotData, total_play_count: u32) -> Result<()> {
    info!("Running startup recent sync...");

    let mut client = MaimaiClient::new(&bot_data.config).wrap_err("create HTTP client")?;
    client
        .ensure_logged_in()
        .await
        .wrap_err("ensure logged in")?;

    let entries = fetch_recent_entries_logged_in(&client)
        .await
        .wrap_err("fetch recent entries")?;

    let entries = annotate_recent_entries_with_play_count(entries, total_play_count);
    let scraped_at = unix_timestamp();
    let count_total = entries.len();
    let count_with_idx = entries
        .iter()
        .filter(|e| e.played_at_unixtime.is_some())
        .count();

    db::upsert_playlogs(&bot_data.db, scraped_at, &entries)
        .await
        .wrap_err("upsert playlogs")?;

    info!(
        "Startup recent sync completed: entries_total={count_total} entries_with_idx={count_with_idx}"
    );
    Ok(())
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

async fn annotate_first_play_flags(
    pool: &SqlitePool,
    entries: &mut [ParsedPlayRecord],
) -> Result<()> {
    for entry in entries {
        if !entry.achievement_new_record {
            continue;
        }
        let Some(diff_category) = entry.diff_category else {
            continue;
        };

        let existing = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT 1
            FROM scores
            WHERE title = ?1
              AND chart_type = ?2
              AND diff_category = ?3
              AND achievement_x10000 IS NOT NULL
            LIMIT 1
            "#,
        )
        .bind(&entry.title)
        .bind(format_chart_type(entry.chart_type))
        .bind(diff_category.as_str())
        .fetch_optional(pool)
        .await
        .wrap_err("check existing score")?;

        if existing.is_none() {
            entry.first_play = true;
        }
    }

    Ok(())
}

fn unix_timestamp() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
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
