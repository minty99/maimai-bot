use eyre::Result;
use tracing::info;

use crate::http_client::is_maintenance_error;
use crate::state::AppState;
use crate::tasks::utils::auth::{build_client, ensure_session};
use crate::tasks::utils::player::fetch_player_data_logged_in;
use crate::tasks::utils::recent::sync_recent_if_play_count_changed;
use crate::tasks::utils::reporting::{SyncCycleReport, log_recent_outcome};
use crate::tasks::utils::scores::SeedScoresOutcome;

pub(crate) type PollingCycleReport = SyncCycleReport;

pub(crate) async fn run_cycle(app_state: &AppState) -> Result<PollingCycleReport> {
    let mut client = build_client(&app_state.config)?;
    if let Err(err) = ensure_session(&mut client).await {
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
    let player_data = match fetch_player_data_logged_in(&mut client).await {
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
    let recent_outcome =
        sync_recent_if_play_count_changed(&app_state.db_pool, &mut client, &player_data).await;

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
