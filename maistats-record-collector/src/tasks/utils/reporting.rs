use tracing::{info, warn};

use crate::tasks::utils::recent::RecentSyncOutcome;
use crate::tasks::utils::scores::SeedScoresOutcome;

#[derive(Debug, Clone, Default)]
pub struct SyncCycleReport {
    pub skipped_for_maintenance: bool,
    pub seeded_scores: SeedScoresOutcome,
    pub recent_outcome: Option<RecentSyncOutcome>,
}

pub(crate) fn log_recent_outcome(scope: &str, outcome: &RecentSyncOutcome) {
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
