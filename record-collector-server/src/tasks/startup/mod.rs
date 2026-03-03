pub(crate) mod bootstrap;

use eyre::Result;
use sqlx::SqlitePool;
use tracing::{info, warn};

use crate::config::RecordCollectorConfig;
use crate::http_client::is_maintenance_window_now;
use crate::tasks::startup::bootstrap::prepare_scores_state;
use crate::tasks::utils::auth::{build_client, ensure_session};
use crate::tasks::utils::detail_hydration::IncompleteBackfillReport;
use crate::tasks::utils::player::fetch_player_data_logged_in;
use crate::tasks::utils::recent::{RecentSyncOutcome, sync_recent_if_play_count_changed};
use crate::tasks::utils::scores::SeedScoresOutcome;

#[derive(Debug, Clone, Default)]
pub(crate) struct StartupSyncReport {
    pub(crate) skipped_for_maintenance: bool,
    pub(crate) seeded_scores: SeedScoresOutcome,
    pub(crate) incomplete_backfill: IncompleteBackfillReport,
    pub(crate) recent_outcome: Option<RecentSyncOutcome>,
}

pub(crate) async fn startup_sync(
    db_pool: &SqlitePool,
    config: &RecordCollectorConfig,
) -> Result<StartupSyncReport> {
    info!("Starting startup sync...");

    if is_maintenance_window_now() {
        info!("Skipping startup sync due to maintenance window (04:00-07:00 local time)");
        return Ok(StartupSyncReport {
            skipped_for_maintenance: true,
            ..StartupSyncReport::default()
        });
    }

    let mut client = build_client(config)?;
    ensure_session(&mut client).await?;

    let (seeded_scores, incomplete_backfill) = prepare_scores_state(db_pool, &mut client).await?;
    let player_data = fetch_player_data_logged_in(&mut client).await?;
    let recent_outcome =
        sync_recent_if_play_count_changed(db_pool, &mut client, &player_data).await;

    log_recent_outcome("startup", &recent_outcome);
    info!(
        "Startup sync complete: seeded={} rows_written={} incomplete_checked={} incomplete_attempted={} incomplete_updated={} incomplete_failed={}",
        seeded_scores.seeded,
        seeded_scores.rows_written,
        incomplete_backfill.checked,
        incomplete_backfill.attempted,
        incomplete_backfill.updated_rows,
        incomplete_backfill.failed_targets.len()
    );

    Ok(StartupSyncReport {
        skipped_for_maintenance: false,
        seeded_scores,
        incomplete_backfill,
        recent_outcome: Some(recent_outcome),
    })
}

fn log_recent_outcome(scope: &str, outcome: &RecentSyncOutcome) {
    match outcome {
        RecentSyncOutcome::SkippedUnchanged => {
            info!("{scope} recent sync skipped: play count unchanged");
        }
        RecentSyncOutcome::SeededWithoutPriorSnapshot {
            inserted_playlogs,
            refreshed_scores,
            failed_targets,
        } => {
            info!(
                "{scope} recent sync seeded without prior snapshot: playlogs={} refreshed_scores={} failed_targets={}",
                inserted_playlogs, refreshed_scores, failed_targets
            );
        }
        RecentSyncOutcome::Updated {
            inserted_playlogs,
            refreshed_scores,
            failed_targets,
        } => {
            info!(
                "{scope} recent sync updated: playlogs={} refreshed_scores={} failed_targets={}",
                inserted_playlogs, refreshed_scores, failed_targets
            );
        }
        RecentSyncOutcome::FailedValidation(message) => {
            warn!("{scope} recent sync validation failed: {message}");
        }
        RecentSyncOutcome::FailedRequest(message) => {
            warn!("{scope} recent sync request failed: {message}");
        }
    }
}
