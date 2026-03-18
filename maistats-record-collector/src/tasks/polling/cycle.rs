use eyre::Result;
use sqlx::SqlitePool;
use tracing::info;

use crate::http_client::is_maintenance_error;
use crate::state::AppState;
use crate::tasks::utils::auth::build_client;
use crate::tasks::utils::recent::sync_recent_if_play_count_changed;
use crate::tasks::utils::reporting::{SyncCycleReport, log_recent_outcome};
use crate::tasks::utils::scores::SeedScoresOutcome;
use crate::tasks::utils::source::CollectorSource;

pub type PollingCycleReport = SyncCycleReport;

pub async fn run_cycle(app_state: &AppState) -> Result<PollingCycleReport> {
    let mut client = build_client(&app_state.config)?;
    run_cycle_with_source(&app_state.db_pool, &mut client).await
}

pub async fn run_cycle_with_source(
    db_pool: &SqlitePool,
    source: &mut impl CollectorSource,
) -> Result<PollingCycleReport> {
    if let Err(err) = source.ensure_session().await {
        if is_maintenance_error(&err) {
            info!(
                "Skipping periodic poll because maimai DX NET is unavailable or under maintenance"
            );
            return Ok(PollingCycleReport {
                skipped_for_maintenance: true,
                ..PollingCycleReport::default()
            });
        }
        return Err(err);
    }

    let seeded_scores = SeedScoresOutcome::default();
    let player_data = match source.fetch_player_data().await {
        Ok(player_data) => player_data,
        Err(err) if is_maintenance_error(&err) => {
            info!(
                "Skipping periodic poll because maimai DX NET is unavailable or under maintenance"
            );
            return Ok(PollingCycleReport {
                skipped_for_maintenance: true,
                ..PollingCycleReport::default()
            });
        }
        Err(err) => return Err(err),
    };
    let recent_outcome = sync_recent_if_play_count_changed(db_pool, source, &player_data).await;

    log_recent_outcome("polling", &recent_outcome);
    info!(
        "Polling cycle complete: seeded={} rows_written={}",
        seeded_scores.seeded, seeded_scores.rows_written
    );

    Ok(PollingCycleReport {
        skipped_for_maintenance: false,
        seeded_scores,
        recent_outcome: Some(recent_outcome),
    })
}
