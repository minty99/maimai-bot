use eyre::Result;
use sqlx::SqlitePool;
use tracing::info;

use crate::config::RecordCollectorConfig;
use crate::http_client::is_maintenance_error;
use crate::tasks::utils::auth::{build_client, ensure_session};
use crate::tasks::utils::player::fetch_player_data_logged_in;
use crate::tasks::utils::recent::sync_recent_if_play_count_changed;
use crate::tasks::utils::reporting::{SyncCycleReport, log_recent_outcome};
use crate::tasks::utils::scores::ensure_scores_seeded;

pub(crate) type StartupSyncReport = SyncCycleReport;

pub(crate) async fn startup_sync(
    db_pool: &SqlitePool,
    config: &RecordCollectorConfig,
) -> Result<StartupSyncReport> {
    info!("Starting startup sync...");

    let mut client = build_client(config)?;
    if let Err(err) = ensure_session(&mut client).await {
        if is_maintenance_error(&err) {
            info!(
                "Skipping startup sync because maimai DX NET is unavailable or under maintenance"
            );
            return Ok(StartupSyncReport {
                skipped_for_maintenance: true,
                ..StartupSyncReport::default()
            });
        }
        return Err(err);
    }

    let seeded_scores = ensure_scores_seeded(db_pool, &mut client).await?;
    let player_data = match fetch_player_data_logged_in(&mut client).await {
        Ok(player_data) => player_data,
        Err(err) if is_maintenance_error(&err) => {
            info!(
                "Skipping startup sync because maimai DX NET is unavailable or under maintenance"
            );
            return Ok(StartupSyncReport {
                skipped_for_maintenance: true,
                ..StartupSyncReport::default()
            });
        }
        Err(err) => return Err(err),
    };
    let recent_outcome =
        sync_recent_if_play_count_changed(db_pool, &mut client, &player_data).await;

    log_recent_outcome("startup", &recent_outcome);
    info!(
        "Startup sync complete: seeded={} rows_written={}",
        seeded_scores.seeded, seeded_scores.rows_written
    );

    Ok(StartupSyncReport {
        skipped_for_maintenance: false,
        seeded_scores,
        recent_outcome: Some(recent_outcome),
    })
}
