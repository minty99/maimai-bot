use eyre::{Result, WrapErr};
use sqlx::SqlitePool;

use crate::db::upsert_playlogs;
use crate::http_client::MaimaiClient;
use crate::tasks::utils::auth::{ExpectedPage, fetch_html_with_auth_recovery};
use crate::tasks::utils::detail_hydration::{
    DetailHydrationReport, refresh_outdated_scores_from_recent,
};
use crate::tasks::utils::player::{
    fetch_player_data_logged_in, load_stored_total_play_count, persist_player_snapshot,
};
use maimai_parsers::parse_recent_html;
use models::{ParsedPlayRecord, ParsedPlayerProfile};

#[derive(Debug, Clone)]
pub(crate) enum RecentSyncOutcome {
    SkippedUnchanged,
    SeededWithoutPriorSnapshot {
        inserted_playlogs: usize,
        refreshed_scores: usize,
        failed_targets: usize,
    },
    Updated {
        inserted_playlogs: usize,
        refreshed_scores: usize,
        failed_targets: usize,
    },
    FailedValidation(String),
    FailedRequest(String),
}

pub(crate) async fn fetch_recent_entries_logged_in(
    client: &mut MaimaiClient,
) -> Result<Vec<ParsedPlayRecord>> {
    let url = reqwest::Url::parse("https://maimaidx-eng.com/maimai-mobile/record/")
        .wrap_err("parse record url")?;
    let html = fetch_html_with_auth_recovery(client, &url, ExpectedPage::Recent).await?;
    parse_recent_html(&html).wrap_err("parse recent html")
}

pub(crate) async fn sync_recent_if_play_count_changed(
    pool: &SqlitePool,
    client: &mut MaimaiClient,
    player_data: &ParsedPlayerProfile,
) -> RecentSyncOutcome {
    let stored_total = match load_stored_total_play_count(pool).await {
        Ok(value) => value,
        Err(err) => return RecentSyncOutcome::FailedRequest(err.to_string()),
    };

    if let Some(stored_total) = stored_total
        && stored_total == player_data.total_play_count
    {
        return match persist_player_snapshot(pool, player_data).await {
            Ok(()) => RecentSyncOutcome::SkippedUnchanged,
            Err(err) => RecentSyncOutcome::FailedRequest(err.to_string()),
        };
    }

    let entries = match fetch_recent_entries_logged_in(client).await {
        Ok(entries) => entries,
        Err(err) => return RecentSyncOutcome::FailedRequest(err.to_string()),
    };
    let entries =
        match annotate_recent_entries_with_credit_id(entries, player_data.total_play_count) {
            Ok(entries) => entries,
            Err(err) => return RecentSyncOutcome::FailedValidation(err.to_string()),
        };

    if let Err(err) = upsert_playlogs(pool, &entries).await {
        return RecentSyncOutcome::FailedRequest(err.to_string());
    }

    let refresh_report = match refresh_outdated_scores_from_recent(pool, client, &entries).await {
        Ok(report) => report,
        Err(err) => return RecentSyncOutcome::FailedRequest(err.to_string()),
    };

    if let Err(err) = persist_player_snapshot(pool, player_data).await {
        return RecentSyncOutcome::FailedRequest(err.to_string());
    }

    outcome_from_refresh_report(stored_total, entries.len(), refresh_report)
}

pub(crate) fn annotate_recent_entries_with_credit_id(
    mut entries: Vec<ParsedPlayRecord>,
    total_play_count: u32,
) -> Result<Vec<ParsedPlayRecord>> {
    let Some(last_track_01_idx) = entries.iter().rposition(|entry| entry.track == Some(1)) else {
        return Err(eyre::eyre!(
            "recent page does not contain TRACK 01; refusing to assign credit_id"
        ));
    };

    entries.truncate(last_track_01_idx + 1);

    let mut credit_idx: u32 = 0;
    for entry in &mut entries {
        entry.credit_id = Some(total_play_count.saturating_sub(credit_idx));
        if entry.track == Some(1) {
            credit_idx = credit_idx.saturating_add(1);
        }
    }

    Ok(entries)
}

fn outcome_from_refresh_report(
    stored_total: Option<u32>,
    inserted_playlogs: usize,
    refresh_report: DetailHydrationReport,
) -> RecentSyncOutcome {
    let refreshed_scores = refresh_report.updated_rows;
    let failed_targets = refresh_report.failed_targets.len();

    if stored_total.is_some() {
        RecentSyncOutcome::Updated {
            inserted_playlogs,
            refreshed_scores,
            failed_targets,
        }
    } else {
        RecentSyncOutcome::SeededWithoutPriorSnapshot {
            inserted_playlogs,
            refreshed_scores,
            failed_targets,
        }
    }
}

pub(crate) async fn fetch_player_and_sync_recent(
    pool: &SqlitePool,
    client: &mut MaimaiClient,
) -> Result<(ParsedPlayerProfile, RecentSyncOutcome)> {
    let player_data = fetch_player_data_logged_in(client)
        .await
        .wrap_err("fetch player data before recent sync")?;
    let recent_outcome = sync_recent_if_play_count_changed(pool, client, &player_data).await;
    Ok((player_data, recent_outcome))
}

#[cfg(test)]
mod tests {
    use super::*;
    use models::ChartType;

    #[test]
    fn annotate_recent_requires_track_01() {
        let entries = vec![ParsedPlayRecord {
            played_at_unixtime: Some(1),
            track: Some(2),
            played_at: Some("2026/01/23 12:34".to_string()),
            credit_id: None,
            title: "Song A".to_string(),
            chart_type: ChartType::Std,
            diff_category: None,
            level: None,
            achievement_percent: None,
            achievement_new_record: false,
            score_rank: None,
            fc: None,
            sync: None,
            dx_score: None,
            dx_score_max: None,
        }];

        assert!(annotate_recent_entries_with_credit_id(entries, 100).is_err());
    }
}
