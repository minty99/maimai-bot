use std::collections::HashMap;

use eyre::{Result, WrapErr};
use reqwest::Url;
use sqlx::SqlitePool;
use tracing::warn;

use crate::db::upsert_scores;
use crate::http_client::MaimaiClient;
use crate::tasks::utils::auth::{ExpectedPage, fetch_html_with_auth_recovery};
use crate::tasks::utils::scores::{
    DetailTarget, ScoreListSnapshot, build_detail_targets, canonical_title_for_detail,
    collect_incomplete_lookup_keys, collect_outdated_lookup_keys, fetch_snapshots_for_lookup_keys,
    reload_score_list_snapshot, score_entries_from_song_detail,
};
use maimai_parsers::parse_song_detail_html;
use models::ParsedPlayRecord;

#[derive(Debug, Clone, Default)]
pub(crate) struct FailedDetailTarget {
    pub(crate) title: String,
    pub(crate) idx: String,
    pub(crate) error: String,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct DetailHydrationReport {
    pub(crate) attempted: usize,
    pub(crate) updated_rows: usize,
    pub(crate) failed_targets: Vec<FailedDetailTarget>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct IncompleteBackfillReport {
    pub(crate) checked: usize,
    pub(crate) attempted: usize,
    pub(crate) updated_rows: usize,
    pub(crate) failed_targets: Vec<FailedDetailTarget>,
}

#[derive(Debug, Default)]
pub(crate) struct DetailPageCache {
    pages: HashMap<String, models::ParsedSongDetail>,
}

impl DetailPageCache {
    fn get(&self, idx: &str) -> Option<models::ParsedSongDetail> {
        self.pages.get(idx).cloned()
    }

    fn insert(&mut self, idx: String, detail: models::ParsedSongDetail) {
        self.pages.insert(idx, detail);
    }
}

pub(crate) async fn backfill_incomplete_scores_if_needed(
    pool: &SqlitePool,
    client: &mut MaimaiClient,
) -> Result<IncompleteBackfillReport> {
    let lookup_keys = collect_incomplete_lookup_keys(pool)
        .await
        .wrap_err("collect incomplete lookup keys")?;
    if lookup_keys.is_empty() {
        return Ok(IncompleteBackfillReport::default());
    }

    let mut snapshots = fetch_snapshots_for_lookup_keys(client, &lookup_keys)
        .await
        .wrap_err("fetch score list snapshots for incomplete backfill")?;
    let targets = build_detail_targets(&lookup_keys, &snapshots);
    let report = hydrate_targets(pool, client, &targets, &mut snapshots)
        .await
        .wrap_err("hydrate incomplete score targets")?;

    Ok(IncompleteBackfillReport {
        checked: lookup_keys.len(),
        attempted: report.attempted,
        updated_rows: report.updated_rows,
        failed_targets: report.failed_targets,
    })
}

pub(crate) async fn refresh_outdated_scores_from_recent(
    pool: &SqlitePool,
    client: &mut MaimaiClient,
    recent_entries: &[ParsedPlayRecord],
) -> Result<DetailHydrationReport> {
    let lookup_keys = collect_outdated_lookup_keys(pool, recent_entries)
        .await
        .wrap_err("collect outdated lookup keys")?;
    if lookup_keys.is_empty() {
        return Ok(DetailHydrationReport::default());
    }

    let mut snapshots = fetch_snapshots_for_lookup_keys(client, &lookup_keys)
        .await
        .wrap_err("fetch score list snapshots for outdated refresh")?;
    let targets = build_detail_targets(&lookup_keys, &snapshots);
    hydrate_targets(pool, client, &targets, &mut snapshots)
        .await
        .wrap_err("hydrate outdated score targets")
}

pub(crate) async fn fetch_song_detail_for_target(
    client: &mut MaimaiClient,
    target: &DetailTarget,
    snapshots: &mut HashMap<u8, ScoreListSnapshot>,
    cache: &mut DetailPageCache,
) -> Result<models::ParsedSongDetail> {
    if let Some(detail) = cache.get(&target.idx) {
        return Ok(detail);
    }

    match fetch_song_detail_by_idx(client, &target.idx).await {
        Ok(detail) => {
            cache.insert(target.idx.clone(), detail.clone());
            Ok(detail)
        }
        Err(first_err) => {
            warn!(
                "musicDetail fetch failed: title='{}' chart='{}' diff='{}' idx={} snapshot_at={} error={:#}",
                target.lookup_key.title,
                target.lookup_key.chart_type.as_str(),
                target.lookup_key.diff_category.as_str(),
                target.idx,
                target.resolved_from_snapshot_at,
                first_err
            );

            reload_score_list_snapshot(client, snapshots, target.lookup_key.diff_as_u8())
                .await
                .wrap_err("reload score list snapshot after detail failure")?;

            let retry_targets =
                build_detail_targets(std::slice::from_ref(&target.lookup_key), snapshots);
            for retry_target in retry_targets {
                if retry_target.idx == target.idx {
                    continue;
                }

                if let Some(detail) = cache.get(&retry_target.idx) {
                    return Ok(detail);
                }

                match fetch_song_detail_by_idx(client, &retry_target.idx).await {
                    Ok(detail) => {
                        cache.insert(retry_target.idx.clone(), detail.clone());
                        return Ok(detail);
                    }
                    Err(retry_err) => warn!(
                        "musicDetail retry after snapshot reload failed: title='{}' chart='{}' diff='{}' idx={} error={:#}",
                        retry_target.lookup_key.title,
                        retry_target.lookup_key.chart_type.as_str(),
                        retry_target.lookup_key.diff_category.as_str(),
                        retry_target.idx,
                        retry_err
                    ),
                }
            }

            Err(first_err).wrap_err_with(|| {
                format!(
                    "failed musicDetail fetch for idx='{}' (title='{}') after score list reload",
                    target.idx, target.lookup_key.title
                )
            })
        }
    }
}

async fn hydrate_targets(
    pool: &SqlitePool,
    client: &mut MaimaiClient,
    targets: &[DetailTarget],
    snapshots: &mut HashMap<u8, ScoreListSnapshot>,
) -> Result<DetailHydrationReport> {
    let mut cache = DetailPageCache::default();
    let mut updates = Vec::new();
    let mut failed_targets = Vec::new();

    for target in targets {
        match fetch_song_detail_for_target(client, target, snapshots, &mut cache).await {
            Ok(detail) => updates.extend(score_entries_from_song_detail(detail)),
            Err(err) => failed_targets.push(FailedDetailTarget {
                title: target.lookup_key.title.clone(),
                idx: target.idx.clone(),
                error: err.to_string(),
            }),
        }
    }

    if !updates.is_empty() {
        upsert_scores(pool, &updates)
            .await
            .wrap_err("upsert hydrated score rows")?;
    }

    Ok(DetailHydrationReport {
        attempted: targets.len(),
        updated_rows: updates.len(),
        failed_targets,
    })
}

async fn fetch_song_detail_by_idx(
    client: &mut MaimaiClient,
    idx: &str,
) -> Result<models::ParsedSongDetail> {
    let url = Url::parse_with_params(
        "https://maimaidx-eng.com/maimai-mobile/record/musicDetail/",
        &[("idx", idx)],
    )
    .wrap_err("build musicDetail url")?;
    let html = fetch_html_with_auth_recovery(
        client,
        &url,
        ExpectedPage::MusicDetail {
            idx: idx.to_string(),
        },
    )
    .await
    .wrap_err("fetch musicDetail html with auth recovery")?;
    let detail = parse_song_detail_html(&html).wrap_err("parse musicDetail page")?;

    let title = canonical_title_for_detail(&detail);
    if title.is_empty() {
        return Err(eyre::eyre!(
            "empty canonical title from musicDetail idx={idx}"
        ));
    }

    Ok(detail)
}
