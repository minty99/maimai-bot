use std::collections::HashMap;
use std::time::Duration;

use eyre::{Result, WrapErr};
use sqlx::SqlitePool;
use tokio::time::sleep;
use tracing::warn;

use crate::db::apply_recent_sync_atomic;
use crate::http_client::MaimaiClient;
use crate::tasks::utils::auth::fetch_html_with_auth_recovery;
use crate::tasks::utils::player::load_stored_total_play_count;
use crate::tasks::utils::scores::score_entries_from_song_detail;
use crate::tasks::utils::song_detail::SongDetailCache;
use crate::tasks::utils::source::CollectorSource;
use crate::tasks::utils::source::ExpectedPage;
use maimai_parsers::parse_recent_html;
use models::{
    ParsedPlayRecord, ParsedPlayerProfile, ParsedPlaylogDetail, ParsedScoreEntry, ParsedSongDetail,
};

const MAX_STALE_SONG_DETAIL_RETRIES: usize = 3;
const STALE_SONG_DETAIL_RETRY_DELAY: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
pub enum RecentSyncOutcome {
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
    source: &mut impl CollectorSource,
    player_data: &ParsedPlayerProfile,
) -> RecentSyncOutcome {
    let stored_total = match load_stored_total_play_count(pool).await {
        Ok(value) => value,
        Err(err) => return RecentSyncOutcome::FailedRequest(format!("{err:#}")),
    };

    if let Some(stored_total) = stored_total
        && stored_total == player_data.total_play_count
    {
        return RecentSyncOutcome::SkippedUnchanged;
    }

    let entries = match source.fetch_recent_entries().await {
        Ok(entries) => entries,
        Err(err) => return RecentSyncOutcome::FailedRequest(format!("{err:#}")),
    };
    let entries =
        match annotate_recent_entries_with_credit_id(entries, player_data.total_play_count) {
            Ok(entries) => entries,
            Err(err) => return RecentSyncOutcome::FailedValidation(format!("{err:#}")),
        };

    let (resolved_entries, score_updates) =
        match resolve_recent_entries_and_collect_score_updates(pool, source, &entries).await {
            Ok(value) => value,
            Err(err) => return RecentSyncOutcome::FailedRequest(format!("{err:#}")),
        };

    let refreshed_scores = score_updates.len();
    let now = unix_timestamp();
    if let Err(err) =
        apply_recent_sync_atomic(pool, &score_updates, &resolved_entries, player_data, now).await
    {
        return RecentSyncOutcome::FailedRequest(format!("{err:#}"));
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
    source: &mut impl CollectorSource,
    entries: &[ParsedPlayRecord],
) -> Result<(Vec<ParsedPlayRecord>, Vec<ParsedScoreEntry>)> {
    let mut detail_cache = SongDetailCache::default();
    let mut resolved_entries = Vec::with_capacity(entries.len());
    let mut songs_to_refresh = HashMap::new();

    for entry in entries {
        let playlog_idx = entry
            .playlog_detail_idx
            .as_deref()
            .ok_or_else(|| eyre::eyre!("recent entry is missing playlogDetail idx"))?;
        let playlog_detail = source
            .fetch_playlog_detail(playlog_idx)
            .await
            .wrap_err("fetch playlogDetail from recent entry")?;
        if titles_mismatch_when_present(&playlog_detail.title, &entry.title) {
            return Err(eyre::eyre!(
                "recent/playlogDetail title mismatch: recent='{}' playlogDetail='{}'",
                entry.title,
                playlog_detail.title
            ));
        }

        let detail =
            fetch_song_detail_for_recent_entry(source, &mut detail_cache, &playlog_detail, entry)
                .await?;

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

    let updates = songs_to_refresh
        .into_values()
        .flat_map(|entries| entries.into_iter())
        .collect::<Vec<_>>();
    Ok((resolved_entries, updates))
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

fn titles_mismatch_when_present(left: &str, right: &str) -> bool {
    let left = left.trim();
    let right = right.trim();

    !left.is_empty() && !right.is_empty() && left != right
}

fn unix_timestamp() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

async fn fetch_song_detail_for_recent_entry(
    source: &mut impl CollectorSource,
    cache: &mut SongDetailCache,
    playlog_detail: &ParsedPlaylogDetail,
    recent: &ParsedPlayRecord,
) -> Result<ParsedSongDetail> {
    if let Some(detail) = cache.get(&playlog_detail.music_detail_idx)
        && !song_detail_is_stale_for_recent(&detail, recent)
    {
        return Ok(detail);
    }

    for attempt in 1..=MAX_STALE_SONG_DETAIL_RETRIES {
        let detail = source
            .fetch_song_detail(&playlog_detail.music_detail_idx)
            .await
            .wrap_err("fetch musicDetail from playlogDetail")?;
        if titles_mismatch_when_present(&detail.title, &playlog_detail.title) {
            return Err(eyre::eyre!(
                "playlogDetail/musicDetail title mismatch: playlogDetail='{}' musicDetail='{}'",
                playlog_detail.title,
                detail.title
            ));
        }
        if !song_detail_is_stale_for_recent(&detail, recent) {
            cache.insert(playlog_detail.music_detail_idx.clone(), detail.clone());
            return Ok(detail);
        }

        let observed_last_played_at = song_detail_last_played_at(&detail, recent)
            .map(str::to_string)
            .unwrap_or_else(|| "missing".to_string());
        if attempt == MAX_STALE_SONG_DETAIL_RETRIES {
            return Err(eyre::eyre!(
                "musicDetail remained stale after {} attempts: title='{}' music_detail_idx='{}' expected_last_played_at='{}' observed_last_played_at='{}'",
                MAX_STALE_SONG_DETAIL_RETRIES,
                recent.title,
                playlog_detail.music_detail_idx,
                recent.played_at.as_deref().unwrap_or("missing"),
                observed_last_played_at
            ));
        }

        warn!(
            "musicDetail stale for recent-triggered score refresh; retrying: title='{}' music_detail_idx='{}' expected_last_played_at='{}' observed_last_played_at='{}' attempt={}/{}",
            recent.title,
            playlog_detail.music_detail_idx,
            recent.played_at.as_deref().unwrap_or("missing"),
            observed_last_played_at,
            attempt,
            MAX_STALE_SONG_DETAIL_RETRIES
        );
        sleep(STALE_SONG_DETAIL_RETRY_DELAY).await;
    }

    Err(eyre::eyre!(
        "unreachable stale musicDetail retry exit for title='{}'",
        recent.title
    ))
}

fn song_detail_is_stale_for_recent(detail: &ParsedSongDetail, recent: &ParsedPlayRecord) -> bool {
    let Some(expected_played_at) = recent.played_at.as_deref() else {
        return false;
    };

    match song_detail_last_played_at(detail, recent) {
        Some(stored_last_played_at) => stored_last_played_at < expected_played_at,
        None => true,
    }
}

fn song_detail_last_played_at<'a>(
    detail: &'a ParsedSongDetail,
    recent: &ParsedPlayRecord,
) -> Option<&'a str> {
    let diff_category = recent.diff_category?;

    detail
        .difficulties
        .iter()
        .find(|difficulty| {
            difficulty.chart_type == recent.chart_type && difficulty.diff_category == diff_category
        })
        .and_then(|difficulty| difficulty.last_played_at.as_deref())
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
    use models::{ChartType, DifficultyCategory, ParsedSongChartDetail, ParsedSongDetail};

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

    #[test]
    fn title_mismatch_check_ignores_missing_values() {
        assert!(!titles_mismatch_when_present("", ""));
        assert!(!titles_mismatch_when_present("Song A", ""));
        assert!(!titles_mismatch_when_present("", "Song A"));
        assert!(!titles_mismatch_when_present("Song A", "Song A"));
        assert!(titles_mismatch_when_present("Song A", "Song B"));
    }

    #[test]
    fn song_detail_staleness_detects_older_last_played() {
        let recent = ParsedPlayRecord {
            played_at_unixtime: Some(1),
            playlog_detail_idx: Some("14,1".to_string()),
            track: Some(1),
            played_at: Some("2026/03/14 12:34".to_string()),
            credit_id: Some(1),
            title: "Song A".to_string(),
            genre: Some("Genre A".to_string()),
            artist: Some("Artist A".to_string()),
            chart_type: ChartType::Dx,
            diff_category: Some(DifficultyCategory::Master),
            level: None,
            achievement_percent: None,
            achievement_new_record: false,
            score_rank: None,
            fc: None,
            sync: None,
            dx_score: None,
            dx_score_max: None,
        };
        let stale_detail = ParsedSongDetail {
            title: "Song A".to_string(),
            genre: Some("Genre A".to_string()),
            artist: "Artist A".to_string(),
            chart_type: ChartType::Dx,
            difficulties: vec![ParsedSongChartDetail {
                diff_category: DifficultyCategory::Master,
                level: "12+".to_string(),
                chart_type: ChartType::Dx,
                achievement_percent: Some(100.0),
                rank: None,
                fc: None,
                sync: None,
                dx_score: Some(1000),
                dx_score_max: Some(1500),
                last_played_at: Some("2026/03/14 12:20".to_string()),
                play_count: Some(5),
            }],
        };
        let fresh_detail = ParsedSongDetail {
            difficulties: vec![ParsedSongChartDetail {
                last_played_at: Some("2026/03/14 12:34".to_string()),
                ..stale_detail.difficulties[0].clone()
            }],
            ..stale_detail.clone()
        };

        assert!(song_detail_is_stale_for_recent(&stale_detail, &recent));
        assert!(!song_detail_is_stale_for_recent(&fresh_detail, &recent));
    }
}
