use std::collections::HashMap;

use eyre::{Result, WrapErr};
use sqlx::SqlitePool;

use crate::db::{upsert_playlogs, upsert_scores};
use crate::http_client::MaimaiClient;
use crate::tasks::utils::auth::{ExpectedPage, fetch_html_with_auth_recovery};
use crate::tasks::utils::player::{load_stored_total_play_count, persist_player_snapshot};
use crate::tasks::utils::playlog_detail::fetch_playlog_detail;
use crate::tasks::utils::scores::score_entries_from_song_detail;
use crate::tasks::utils::song_detail::{SongDetailCache, fetch_song_detail_by_idx};
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
        return RecentSyncOutcome::SkippedUnchanged;
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

    let (resolved_entries, refreshed_scores) =
        match resolve_recent_entries_and_collect_score_updates(pool, client, &entries).await {
            Ok(value) => value,
            Err(err) => return RecentSyncOutcome::FailedRequest(err.to_string()),
        };

    if let Err(err) = upsert_playlogs(pool, &resolved_entries).await {
        return RecentSyncOutcome::FailedRequest(err.to_string());
    }

    if let Err(err) = persist_player_snapshot(pool, player_data).await {
        return RecentSyncOutcome::FailedRequest(err.to_string());
    }

    if stored_total.is_some() {
        RecentSyncOutcome::Updated {
            inserted_playlogs: resolved_entries.len(),
            refreshed_scores,
            failed_targets: 0,
        }
    } else {
        RecentSyncOutcome::SeededWithoutPriorSnapshot {
            inserted_playlogs: resolved_entries.len(),
            refreshed_scores,
            failed_targets: 0,
        }
    }
}

async fn resolve_recent_entries_and_collect_score_updates(
    pool: &SqlitePool,
    client: &mut MaimaiClient,
    entries: &[ParsedPlayRecord],
) -> Result<(Vec<ParsedPlayRecord>, usize)> {
    let mut detail_cache = SongDetailCache::default();
    let mut resolved_entries = Vec::with_capacity(entries.len());
    let mut songs_to_refresh = HashMap::new();

    for entry in entries {
        let playlog_idx = entry
            .playlog_detail_idx
            .as_deref()
            .ok_or_else(|| eyre::eyre!("recent entry is missing playlogDetail idx"))?;
        let playlog_detail = fetch_playlog_detail(client, playlog_idx)
            .await
            .wrap_err("fetch playlogDetail from recent entry")?;
        if playlog_detail.title.trim() != entry.title.trim() {
            return Err(eyre::eyre!(
                "recent/playlogDetail title mismatch: recent='{}' playlogDetail='{}'",
                entry.title,
                playlog_detail.title
            ));
        }

        let detail = if let Some(cached) = detail_cache.get(&playlog_detail.music_detail_idx) {
            cached
        } else {
            let detail = fetch_song_detail_by_idx(client, &playlog_detail.music_detail_idx)
                .await
                .wrap_err("fetch musicDetail from playlogDetail")?;
            if detail.title.trim() != playlog_detail.title.trim() {
                return Err(eyre::eyre!(
                    "playlogDetail/musicDetail title mismatch: playlogDetail='{}' musicDetail='{}'",
                    playlog_detail.title,
                    detail.title
                ));
            }
            detail_cache.insert(playlog_detail.music_detail_idx.clone(), detail.clone());
            detail
        };

        let mut resolved = entry.clone();
        resolved.genre = detail.genre.clone();
        resolved.artist = Some(detail.artist.clone());

        if score_row_is_affected(pool, &resolved).await? {
            songs_to_refresh
                .entry((
                    detail.title.clone(),
                    detail.genre.clone().unwrap_or_default(),
                    detail.artist.clone(),
                ))
                .or_insert_with(|| score_entries_from_song_detail(detail.clone()));
        }

        resolved_entries.push(resolved);
    }

    if !songs_to_refresh.is_empty() {
        let updates = songs_to_refresh
            .into_values()
            .flat_map(|entries| entries.into_iter())
            .collect::<Vec<_>>();
        upsert_scores(pool, &updates)
            .await
            .wrap_err("upsert affected song detail rows")?;
        Ok((resolved_entries, updates.len()))
    } else {
        Ok((resolved_entries, 0))
    }
}

async fn score_row_is_affected(pool: &SqlitePool, recent: &ParsedPlayRecord) -> Result<bool> {
    let Some(diff_category) = recent.diff_category else {
        return Ok(true);
    };
    let Some(played_at) = recent.played_at.as_deref() else {
        return Ok(true);
    };
    let genre = recent.genre.as_deref().unwrap_or("");
    let artist = recent.artist.as_deref().unwrap_or("");

    let stored_last_played_at = sqlx::query_scalar::<_, Option<String>>(
        r#"
        SELECT last_played_at
        FROM scores
        WHERE title = ?1
          AND genre = ?2
          AND artist = ?3
          AND chart_type = ?4
          AND diff_category = ?5
        "#,
    )
    .bind(&recent.title)
    .bind(genre)
    .bind(artist)
    .bind(recent.chart_type.as_str())
    .bind(diff_category.as_str())
    .fetch_optional(pool)
    .await
    .wrap_err("fetch stored last_played_at for affected song check")?
    .flatten();

    Ok(match stored_last_played_at.as_deref() {
        Some(stored) => stored < played_at,
        None => true,
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use models::ChartType;

    #[test]
    fn annotate_recent_requires_track_01() {
        let entries = vec![ParsedPlayRecord {
            played_at_unixtime: Some(1),
            playlog_detail_idx: Some("14,1".to_string()),
            track: Some(2),
            played_at: Some("2026/01/23 12:34".to_string()),
            credit_id: None,
            title: "Song A".to_string(),
            genre: None,
            artist: None,
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

    #[tokio::test]
    async fn score_row_is_affected_when_missing_or_older() -> eyre::Result<()> {
        let pool = crate::db::connect("sqlite::memory:").await?;
        crate::db::migrate(&pool).await?;

        let recent = ParsedPlayRecord {
            played_at_unixtime: Some(1),
            playlog_detail_idx: Some("14,1".to_string()),
            track: Some(1),
            played_at: Some("2026/01/23 12:34".to_string()),
            credit_id: Some(1),
            title: "Song A".to_string(),
            genre: Some("Genre A".to_string()),
            artist: Some("Artist A".to_string()),
            chart_type: ChartType::Dx,
            diff_category: Some(models::DifficultyCategory::Master),
            level: None,
            achievement_percent: None,
            achievement_new_record: false,
            score_rank: None,
            fc: None,
            sync: None,
            dx_score: None,
            dx_score_max: None,
        };
        assert!(score_row_is_affected(&pool, &recent).await?);
        Ok(())
    }
}
