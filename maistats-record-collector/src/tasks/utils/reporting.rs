use tracing::{info, warn};

use crate::tasks::utils::recent::RecentSyncOutcome;
use crate::tasks::utils::scores::SeedScoresOutcome;

#[derive(Debug, Clone, Default)]
pub struct SyncCycleReport {
    pub skipped_for_maintenance: bool,
    pub seeded: bool,
    pub seeded_rows_written: usize,
    pub recent_outcome: Option<RecentSyncOutcome>,
}

impl From<SeedScoresOutcome> for SyncCycleReport {
    fn from(value: SeedScoresOutcome) -> Self {
        Self {
            skipped_for_maintenance: false,
            seeded: value.seeded,
            seeded_rows_written: value.rows_written,
            recent_outcome: None,
        }
    }
}

pub(crate) fn log_recent_outcome(scope: &str, outcome: &RecentSyncOutcome) {
    match outcome {
        RecentSyncOutcome::SkippedUnchanged => {
            info!("{scope} recent sync skipped: play count unchanged or no new recent credits");
        }
        RecentSyncOutcome::Updated {
            inserted_credits,
            inserted_playlogs,
            refreshed_scores,
            failed_targets,
        } => {
            info!(
                "{scope} recent sync updated: credits={} playlogs={} refreshed_scores={} failed_targets={}",
                inserted_credits, inserted_playlogs, refreshed_scores, failed_targets
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
