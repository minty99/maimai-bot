use eyre::Result;
use sqlx::SqlitePool;

use crate::http_client::MaimaiClient;
use crate::tasks::utils::detail_hydration::{
    IncompleteBackfillReport, backfill_incomplete_scores_if_needed,
};
use crate::tasks::utils::scores::{SeedScoresOutcome, ensure_scores_seeded};

pub(crate) async fn prepare_scores_state(
    pool: &SqlitePool,
    client: &mut MaimaiClient,
) -> Result<(SeedScoresOutcome, IncompleteBackfillReport)> {
    let seed_outcome = ensure_scores_seeded(pool, client).await?;
    let incomplete_backfill = backfill_incomplete_scores_if_needed(pool, client).await?;
    Ok((seed_outcome, incomplete_backfill))
}
