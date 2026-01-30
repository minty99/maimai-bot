use std::time::Duration;

use eyre::{Result, WrapErr};
use tokio::time::interval;
use tracing::{debug, error, info};

use crate::state::AppState;

const STATE_KEY_TOTAL_PLAY_COUNT: &str = "player.total_play_count";
const STATE_KEY_RATING: &str = "player.rating";

pub fn start_background_polling(app_state: AppState) {
    tokio::spawn(async move {
        let mut timer = interval(Duration::from_secs(600));
        timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        info!("Background polling started: periodic playerData poll (every 10 minutes)");

        loop {
            timer.tick().await;

            info!("Running periodic playerData poll...");

            match poll_and_sync_if_needed(&app_state).await {
                Ok(true) => info!("New plays detected; refreshed DB"),
                Ok(false) => {}
                Err(e) => error!("Periodic poll failed: {e:#}"),
            }
        }
    });
}

async fn poll_and_sync_if_needed(app_state: &AppState) -> Result<bool> {
    if maimai_bot::http::is_maintenance_window_now() {
        info!("Skipping periodic poll due to maintenance window (04:00-07:00 local time)");
        return Ok(false);
    }

    let mut client = maimai_bot::http::MaimaiClient::new(&app_state.config)
        .wrap_err("create HTTP client")?;
    client
        .ensure_logged_in()
        .await
        .wrap_err("ensure logged in")?;

    let player_data = fetch_player_data_logged_in(&client)
        .await
        .wrap_err("fetch player data")?;

    let stored_total = maimai_bot::db::get_app_state_u32(&app_state.db_pool, STATE_KEY_TOTAL_PLAY_COUNT)
        .await
        .ok()
        .flatten();

    if let Some(stored_total) = stored_total
        && stored_total == player_data.total_play_count
    {
        debug!("No play count change detected (stored={stored_total}, current={})", player_data.total_play_count);
        return Ok(false);
    }

    info!("Play count changed (stored={:?}, current={}); syncing recent playlogs", stored_total, player_data.total_play_count);

    let entries = fetch_recent_entries_logged_in(&client)
        .await
        .wrap_err("fetch recent")?;

    let mut entries = annotate_recent_entries_with_play_count(entries, player_data.total_play_count);

    if stored_total.is_some() {
        annotate_first_play_flags(&app_state.db_pool, &mut entries)
            .await
            .wrap_err("classify first plays")?;
    }

    let scraped_at = unix_timestamp();

    maimai_bot::db::upsert_playlogs(&app_state.db_pool, scraped_at, &entries)
        .await
        .wrap_err("upsert playlogs")?;

    persist_player_snapshot(&app_state.db_pool, &player_data)
        .await
        .wrap_err("persist player snapshot")?;

    if stored_total.is_some() {
        Ok(true)
    } else {
        debug!("No stored total play count; seeded DB without triggering notification");
        Ok(false)
    }
}

async fn fetch_player_data_logged_in(
    client: &maimai_bot::http::MaimaiClient,
) -> Result<maimai_bot::maimai::models::ParsedPlayerData> {
    let url = reqwest::Url::parse("https://maimaidx-eng.com/maimai-mobile/playerData/")
        .wrap_err("parse playerData url")?;
    let bytes = client
        .get_bytes(&url)
        .await
        .wrap_err("fetch playerData url")?;
    let html = String::from_utf8(bytes).wrap_err("playerData response is not utf-8")?;
    maimai_bot::maimai::parse::player_data::parse_player_data_html(&html)
        .wrap_err("parse playerData html")
}

async fn fetch_recent_entries_logged_in(
    client: &maimai_bot::http::MaimaiClient,
) -> Result<Vec<maimai_bot::maimai::models::ParsedPlayRecord>> {
    let url = reqwest::Url::parse("https://maimaidx-eng.com/maimai-mobile/record/")
        .wrap_err("parse record url")?;
    let bytes = client.get_bytes(&url).await.wrap_err("fetch record url")?;
    let html = String::from_utf8(bytes).wrap_err("record response is not utf-8")?;
    maimai_bot::maimai::parse::recent::parse_recent_html(&html).wrap_err("parse recent html")
}

fn annotate_recent_entries_with_play_count(
    mut entries: Vec<maimai_bot::maimai::models::ParsedPlayRecord>,
    total_play_count: u32,
) -> Vec<maimai_bot::maimai::models::ParsedPlayRecord> {
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
    pool: &maimai_bot::db::SqlitePool,
    entries: &mut [maimai_bot::maimai::models::ParsedPlayRecord],
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
        .bind(maimai_bot::db::format_chart_type(entry.chart_type))
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

async fn persist_player_snapshot(
    pool: &maimai_bot::db::SqlitePool,
    player_data: &maimai_bot::maimai::models::ParsedPlayerData,
) -> Result<()> {
    let now = unix_timestamp();
    maimai_bot::db::set_app_state_u32(
        pool,
        STATE_KEY_TOTAL_PLAY_COUNT,
        player_data.total_play_count,
        now,
    )
    .await
    .wrap_err("store total play count")?;
    maimai_bot::db::set_app_state_u32(pool, STATE_KEY_RATING, player_data.rating, now)
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
