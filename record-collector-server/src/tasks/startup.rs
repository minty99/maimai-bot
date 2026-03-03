use eyre::{Result, WrapErr};
use tracing::{info, warn};

use crate::backup::service::BackupReason;
use crate::db::{count_scores_rows, get_app_state_u32, upsert_playlogs};
use crate::http_client::{MaimaiClient, is_maintenance_window_now};
use crate::state::AppState;
use crate::tasks::scores_sync::{
    bootstrap_scores_with_client, refresh_incomplete_scores_with_client,
    refresh_outdated_scores_from_recent,
};
use crate::tasks::sync_shared::{
    STATE_KEY_TOTAL_PLAY_COUNT, annotate_recent_entries_with_play_count,
    fetch_player_data_logged_in, fetch_recent_entries_logged_in, persist_player_snapshot,
    to_app_config,
};

pub(crate) async fn startup_sync(app_state: &AppState) -> Result<()> {
    info!("Starting startup sync...");

    if is_maintenance_window_now() {
        info!("Skipping startup sync due to maintenance window (04:00-07:00 local time)");
        return Ok(());
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
    let mut should_request_backup = false;
    if score_rows == 0 {
        let bootstrap_count = bootstrap_scores_with_client(&app_state.db_pool, &client)
            .await
            .wrap_err("bootstrap scores")?;
        info!("Scores bootstrap completed because table was empty: rows={bootstrap_count}");
        should_request_backup |= bootstrap_count > 0;
    }

    match refresh_incomplete_scores_with_client(&app_state.db_pool, &client).await {
        Ok(backfilled_rows) => {
            info!("Incomplete scores backfill at startup: rows={backfilled_rows}");
            should_request_backup |= backfilled_rows > 0;
        }
        Err(e) => warn!("Incomplete scores backfill failed at startup; continuing: {e:#}"),
    }

    let player_data = fetch_player_data_logged_in(&client)
        .await
        .wrap_err("fetch player data")?;

    info!(
        "Player data fetched: user_name={}, total_play_count={}, rating={}",
        player_data.user_name, player_data.total_play_count, player_data.rating
    );

    let stored_total = get_app_state_u32(&app_state.db_pool, STATE_KEY_TOTAL_PLAY_COUNT)
        .await
        .ok()
        .flatten();

    let should_sync_recent = match stored_total {
        Some(v) if v == player_data.total_play_count => {
            info!("Play count unchanged ({}); skipping recent sync", v);
            false
        }
        Some(v) => {
            info!(
                "Play count changed: {} -> {}; will sync recent",
                v, player_data.total_play_count
            );
            true
        }
        None => {
            info!("No stored play count; will perform initial recent sync");
            true
        }
    };

    if should_sync_recent {
        let entries = fetch_recent_entries_logged_in(&client)
            .await
            .wrap_err("fetch recent entries")?;
        let entries =
            annotate_recent_entries_with_play_count(entries, player_data.total_play_count);

        let inserted_playlogs = upsert_playlogs(&app_state.db_pool, &entries)
            .await
            .wrap_err("upsert playlogs")?;
        should_request_backup |= inserted_playlogs > 0;

        let refreshed_rows =
            refresh_outdated_scores_from_recent(&app_state.db_pool, &client, &entries)
                .await
                .wrap_err("refresh outdated scores from recent")?;
        info!("Outdated scores refreshed from recent: rows={refreshed_rows}");
        should_request_backup |= refreshed_rows > 0;
    }

    persist_player_snapshot(&app_state.db_pool, &player_data)
        .await
        .wrap_err("persist player snapshot")?;

    if should_request_backup && let Some(backup_service) = &app_state.backup_service {
        backup_service.request_backup(
            BackupReason::StartupSync,
            Some(player_data.total_play_count),
        );
    }

    info!("Startup sync complete");
    Ok(())
}
