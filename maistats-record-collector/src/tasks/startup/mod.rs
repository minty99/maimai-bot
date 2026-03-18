use eyre::Result;
use sqlx::SqlitePool;
use tracing::info;

use crate::config::RecordCollectorConfig;
use crate::http_client::is_maintenance_error;
use crate::tasks::utils::auth::build_client;
use crate::tasks::utils::recent::sync_recent_if_play_count_changed;
use crate::tasks::utils::reporting::{SyncCycleReport, log_recent_outcome};
use crate::tasks::utils::scores::ensure_scores_seeded;
use crate::tasks::utils::source::CollectorSource;

pub type StartupSyncReport = SyncCycleReport;

pub(crate) async fn startup_sync(
    db_pool: &SqlitePool,
    config: &RecordCollectorConfig,
) -> Result<StartupSyncReport> {
    info!("Starting startup sync...");

    let mut client = build_client(config)?;
    startup_sync_with_source(db_pool, &mut client).await
}

pub async fn startup_sync_with_source(
    db_pool: &SqlitePool,
    source: &mut impl CollectorSource,
) -> Result<StartupSyncReport> {
    if let Err(err) = source.ensure_session().await {
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

    let seeded_scores = ensure_scores_seeded(db_pool, source).await?;
    let player_data = match source.fetch_player_data().await {
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
    let recent_outcome = sync_recent_if_play_count_changed(db_pool, source, &player_data).await;

    log_recent_outcome("startup", &recent_outcome);
    info!(
        "Startup sync complete: seeded={} rows_written={}",
        seeded_scores.seeded, seeded_scores.rows_written
    );

    Ok(StartupSyncReport {
        skipped_for_maintenance: false,
        seeded: seeded_scores.seeded,
        seeded_rows_written: seeded_scores.rows_written,
        recent_outcome: Some(recent_outcome),
    })
}
