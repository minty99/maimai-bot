use eyre::Result;
use sqlx::SqlitePool;

use crate::http_client::MaimaiClient;
use crate::tasks::utils::scores::{
    SeedScoresOutcome, backfill_missing_playlog_metadata, ensure_scores_seeded,
};

pub(crate) async fn prepare_scores_state(
    pool: &SqlitePool,
    client: &mut MaimaiClient,
) -> Result<(SeedScoresOutcome, usize)> {
    let seed_outcome = ensure_scores_seeded(pool, client).await?;
    let playlog_metadata_backfilled = backfill_missing_playlog_metadata(pool).await?;
    Ok((seed_outcome, playlog_metadata_backfilled))
}
