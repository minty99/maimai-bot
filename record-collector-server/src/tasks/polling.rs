use std::time::Duration;

use eyre::{Result, WrapErr};
use tokio::time::interval;
use tracing::{debug, error, info};

use crate::db::{count_scores_rows, get_app_state_u32, upsert_playlogs};
use crate::http_client::{MaimaiClient, is_maintenance_window_now};
use crate::state::AppState;
use crate::tasks::scores_sync::{
    bootstrap_scores_with_client, refresh_outdated_scores_from_recent,
};
use crate::tasks::sync_shared::{
    STATE_KEY_TOTAL_PLAY_COUNT, annotate_recent_entries_with_play_count,
    fetch_player_data_logged_in, fetch_recent_entries_logged_in, persist_player_snapshot,
    to_app_config,
};
use models::{ParsedPlayRecord, SongTitle};

pub(crate) fn start_background_polling(app_state: AppState) {
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
    if is_maintenance_window_now() {
        info!("Skipping periodic poll due to maintenance window (04:00-07:00 local time)");
        return Ok(false);
    }

    let app_config = to_app_config(&app_state.config);
    let mut client = MaimaiClient::new(&app_config).wrap_err("create HTTP client")?;
    client
        .ensure_logged_in()
        .await
        .wrap_err("ensure logged in")?;

    let score_rows = count_scores_rows(&app_state.db_pool)
        .await
        .wrap_err("count scores rows")?;
    if score_rows == 0 {
        let bootstrap_count = bootstrap_scores_with_client(&app_state.db_pool, &client)
            .await
            .wrap_err("bootstrap scores")?;
        info!("Scores bootstrap completed because table was empty: rows={bootstrap_count}");
    }

    let player_data = fetch_player_data_logged_in(&client)
        .await
        .wrap_err("fetch player data")?;

    let stored_total = get_app_state_u32(&app_state.db_pool, STATE_KEY_TOTAL_PLAY_COUNT)
        .await
        .ok()
        .flatten();

    if let Some(stored_total) = stored_total
        && stored_total == player_data.total_play_count
    {
        debug!(
            "No play count change detected (stored={stored_total}, current={})",
            player_data.total_play_count
        );
        return Ok(false);
    }

    info!(
        "Play count changed (stored={:?}, current={}); syncing recent playlogs",
        stored_total, player_data.total_play_count
    );

    let entries = fetch_recent_entries_logged_in(&client)
        .await
        .wrap_err("fetch recent")?;

    let mut entries =
        annotate_recent_entries_with_play_count(entries, player_data.total_play_count);

    if stored_total.is_some() {
        annotate_first_play_flags(&app_state.db_pool, &mut entries)
            .await
            .wrap_err("classify first plays")?;
    }

    upsert_playlogs(&app_state.db_pool, &entries)
        .await
        .wrap_err("upsert playlogs")?;

    let refreshed_rows = refresh_outdated_scores_from_recent(&app_state.db_pool, &client, &entries)
        .await
        .wrap_err("refresh outdated scores from recent")?;
    info!("Outdated scores refreshed from recent: rows={refreshed_rows}");

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

async fn annotate_first_play_flags(
    pool: &sqlx::SqlitePool,
    entries: &mut [ParsedPlayRecord],
) -> Result<()> {
    for entry in entries {
        if !entry.achievement_new_record {
            continue;
        }
        if SongTitle::parse(&entry.title).is_ambiguous_unqualified() {
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
        .bind(entry.chart_type.as_str())
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
