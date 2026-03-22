use std::collections::{HashMap, HashSet};

use eyre::{Result, WrapErr};
use sqlx::SqlitePool;

use crate::db::{apply_recent_sync_atomic, store_player_profile_snapshot};
use crate::http_client::MaimaiClient;
use crate::tasks::utils::auth::fetch_html_with_auth_recovery;
use crate::tasks::utils::player::load_stored_player_profile_state;
use crate::tasks::utils::scores::score_entries_from_song_detail;
use crate::tasks::utils::song_detail::SongDetailCache;
use crate::tasks::utils::source::CollectorSource;
use crate::tasks::utils::source::ExpectedPage;
use maimai_parsers::parse_recent_html;
use models::{
    ParsedPlayRecord, ParsedPlayerProfile, ParsedPlaylogDetail, ParsedScoreEntry, ParsedSongDetail,
};

#[derive(Debug, Clone)]
pub enum RecentSyncOutcome {
    SkippedUnchanged,
    Updated {
        inserted_credits: usize,
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
    let stored_player_state = match load_stored_player_profile_state(pool).await {
        Ok(value) => value,
        Err(err) => return RecentSyncOutcome::FailedRequest(format!("{err:#}")),
    };
    let stored_total = stored_player_state.total_play_count();

    if let Some(stored_total) = stored_total
        && stored_total == player_data.total_play_count
    {
        if stored_player_state.has_incomplete_fields() {
            // Upgrade/backfill path for older DBs that predate newer player snapshot fields.
            return backfill_incomplete_player_snapshot_or_fail(pool, player_data).await;
        }

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

    let new_entries = match filter_entries_for_new_credits(pool, &entries).await {
        Ok(entries) => entries,
        Err(err) => return RecentSyncOutcome::FailedRequest(format!("{err:#}")),
    };

    if new_entries.is_empty() {
        let now = unix_timestamp();
        return match store_player_profile_snapshot(pool, player_data, now).await {
            Ok(()) => RecentSyncOutcome::SkippedUnchanged,
            Err(err) => RecentSyncOutcome::FailedRequest(format!("{err:#}")),
        };
    }

    let (resolved_entries, score_updates) =
        match resolve_recent_entries_and_collect_score_updates(source, &new_entries).await {
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

    RecentSyncOutcome::Updated {
        inserted_credits: count_distinct_credit_ids(&resolved_entries),
        inserted_playlogs: resolved_entries.len(),
        refreshed_scores,
        failed_targets: 0,
    }
}

async fn filter_entries_for_new_credits(
    pool: &SqlitePool,
    entries: &[ParsedPlayRecord],
) -> Result<Vec<ParsedPlayRecord>> {
    let candidate_credit_ids = entries
        .iter()
        .filter_map(|entry| entry.credit_id)
        .collect::<HashSet<_>>();
    if candidate_credit_ids.is_empty() {
        return Ok(entries.to_vec());
    }

    let mut existing_credit_ids = HashSet::new();
    for credit_id in candidate_credit_ids {
        let exists = sqlx::query_scalar::<_, i64>(
            "SELECT EXISTS(SELECT 1 FROM playlogs WHERE credit_id = ?1 LIMIT 1)",
        )
        .bind(i64::from(credit_id))
        .fetch_one(pool)
        .await
        .wrap_err("check existing credit_id in playlogs")?;
        if exists != 0 {
            existing_credit_ids.insert(credit_id);
        }
    }

    Ok(entries
        .iter()
        .filter(|entry| {
            entry
                .credit_id
                .map(|credit_id| !existing_credit_ids.contains(&credit_id))
                .unwrap_or(true)
        })
        .cloned()
        .collect())
}

async fn resolve_recent_entries_and_collect_score_updates(
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
            fetch_song_detail_for_recent_entry(source, &mut detail_cache, &playlog_detail).await?;

        let mut resolved = entry.clone();
        resolved.genre = detail.genre.clone();
        resolved.artist = Some(detail.artist.clone());

        songs_to_refresh
            .entry((
                detail.title.clone(),
                detail.genre.clone().unwrap_or_default(),
                detail.artist.clone(),
            ))
            .or_insert_with(|| score_entries_from_song_detail(detail.clone()));

        resolved_entries.push(resolved);
    }

    let updates = songs_to_refresh
        .into_values()
        .flat_map(|entries| entries.into_iter())
        .collect::<Vec<_>>();
    Ok((resolved_entries, updates))
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

fn count_distinct_credit_ids(entries: &[ParsedPlayRecord]) -> usize {
    entries
        .iter()
        .filter_map(|entry| entry.credit_id)
        .collect::<HashSet<_>>()
        .len()
}

async fn backfill_incomplete_player_snapshot_or_fail(
    pool: &SqlitePool,
    player_data: &ParsedPlayerProfile,
) -> RecentSyncOutcome {
    let now = unix_timestamp();
    match store_player_profile_snapshot(pool, player_data, now).await {
        Ok(()) => RecentSyncOutcome::SkippedUnchanged,
        Err(err) => RecentSyncOutcome::FailedRequest(format!("{err:#}")),
    }
}

async fn fetch_song_detail_for_recent_entry(
    source: &mut impl CollectorSource,
    cache: &mut SongDetailCache,
    playlog_detail: &ParsedPlaylogDetail,
) -> Result<ParsedSongDetail> {
    if let Some(detail) = cache.get(&playlog_detail.music_detail_idx) {
        return Ok(detail);
    }

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

    cache.insert(playlog_detail.music_detail_idx.clone(), detail.clone());
    Ok(detail)
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
    use crate::tasks::utils::source::{FixtureCollectorData, FixtureCollectorSource};
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
    async fn recent_entries_always_refresh_scores_even_when_played_at_matches_existing()
    -> eyre::Result<()> {
        let pool = crate::db::connect("sqlite::memory:").await?;
        crate::db::migrate(&pool).await?;
        crate::db::upsert_scores(
            &pool,
            &[models::ParsedScoreEntry {
                title: "Song A".to_string(),
                genre: "Genre A".to_string(),
                artist: "Artist A".to_string(),
                chart_type: ChartType::Dx,
                diff_category: DifficultyCategory::Master,
                level: "12+".to_string(),
                achievement_percent: Some(99.0),
                rank: Some("SS".parse().unwrap()),
                fc: Some("FC".parse().unwrap()),
                sync: Some("FS".parse().unwrap()),
                dx_score: Some(1980),
                dx_score_max: Some(2100),
                last_played_at: Some("2026/03/05 22:03".to_string()),
                play_count: Some(10),
                source_idx: None,
            }],
        )
        .await?;

        let recent = ParsedPlayRecord {
            played_at_unixtime: Some(1),
            playlog_detail_idx: Some("song-a::1".to_string()),
            track: Some(1),
            played_at: Some("2026/03/05 22:03".to_string()),
            credit_id: Some(1),
            title: "Song A".to_string(),
            genre: None,
            artist: None,
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
        let detail = ParsedSongDetail {
            title: "Song A".to_string(),
            genre: Some("Genre A".to_string()),
            artist: "Artist A".to_string(),
            chart_type: ChartType::Dx,
            difficulties: vec![ParsedSongChartDetail {
                diff_category: DifficultyCategory::Master,
                level: "12+".to_string(),
                chart_type: ChartType::Dx,
                achievement_percent: Some(100.2),
                rank: Some("SSS".parse().unwrap()),
                fc: Some("AP".parse().unwrap()),
                sync: Some("FDX+".parse().unwrap()),
                dx_score: Some(2050),
                dx_score_max: Some(2100),
                last_played_at: Some("2026/03/05 22:15".to_string()),
                play_count: Some(11),
            }],
        };
        let mut source = FixtureCollectorSource::from_data(FixtureCollectorData {
            player_data: None,
            recent_entries: Some(vec![recent]),
            score_lists: Default::default(),
            playlog_details: Default::default(),
            song_details: std::collections::BTreeMap::from([("song-a".to_string(), detail)]),
        });
        let entries = source.fetch_recent_entries().await?;

        let (_, score_updates) =
            resolve_recent_entries_and_collect_score_updates(&mut source, &entries).await?;

        assert_eq!(score_updates.len(), 1);
        assert_eq!(score_updates[0].title, "Song A");
        assert_eq!(
            score_updates[0].last_played_at.as_deref(),
            Some("2026/03/05 22:15")
        );
        Ok(())
    }

    #[tokio::test]
    async fn filters_out_entries_for_existing_credits_before_detail_fetch() -> eyre::Result<()> {
        let pool = crate::db::connect("sqlite::memory:").await?;
        crate::db::migrate(&pool).await?;
        crate::db::apply_recent_sync_atomic(
            &pool,
            &[],
            &[ParsedPlayRecord {
                played_at_unixtime: Some(100),
                playlog_detail_idx: Some("old-song::100".to_string()),
                track: Some(1),
                played_at: Some("2026/03/05 21:00".to_string()),
                credit_id: Some(10),
                title: "Old Song".to_string(),
                genre: Some("Genre".to_string()),
                artist: Some("Artist".to_string()),
                chart_type: ChartType::Std,
                diff_category: Some(DifficultyCategory::Basic),
                level: Some("4".to_string()),
                achievement_percent: Some(90.0),
                achievement_new_record: false,
                score_rank: Some("AA".parse().unwrap()),
                fc: None,
                sync: None,
                dx_score: Some(900),
                dx_score_max: Some(1000),
            }],
            &ParsedPlayerProfile {
                user_name: "fixture-user".to_string(),
                rating: 10_000,
                current_version_play_count: 10,
                total_play_count: 10,
            },
            1,
        )
        .await?;

        let old_entry = ParsedPlayRecord {
            played_at_unixtime: Some(100),
            playlog_detail_idx: Some("old-song::100".to_string()),
            track: Some(1),
            played_at: Some("2026/03/05 21:00".to_string()),
            credit_id: Some(10),
            title: "Old Song".to_string(),
            genre: None,
            artist: None,
            chart_type: ChartType::Std,
            diff_category: Some(DifficultyCategory::Basic),
            level: Some("4".to_string()),
            achievement_percent: Some(90.0),
            achievement_new_record: false,
            score_rank: Some("AA".parse().unwrap()),
            fc: None,
            sync: None,
            dx_score: Some(900),
            dx_score_max: Some(1000),
        };
        let new_entry = ParsedPlayRecord {
            played_at_unixtime: Some(200),
            playlog_detail_idx: Some("new-song::200".to_string()),
            track: Some(1),
            played_at: Some("2026/03/05 22:00".to_string()),
            credit_id: Some(11),
            title: "New Song".to_string(),
            genre: None,
            artist: None,
            chart_type: ChartType::Std,
            diff_category: Some(DifficultyCategory::Basic),
            level: Some("4".to_string()),
            achievement_percent: Some(95.0),
            achievement_new_record: true,
            score_rank: Some("AAA".parse().unwrap()),
            fc: Some("FC".parse().unwrap()),
            sync: Some("FS".parse().unwrap()),
            dx_score: Some(950),
            dx_score_max: Some(1000),
        };

        let filtered =
            filter_entries_for_new_credits(&pool, &[old_entry.clone(), new_entry.clone()]).await?;
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].credit_id, Some(11));
        assert_eq!(filtered[0].title, "New Song");

        let mut source = FixtureCollectorSource::from_data(FixtureCollectorData {
            player_data: None,
            recent_entries: Some(vec![old_entry, new_entry]),
            score_lists: Default::default(),
            playlog_details: Default::default(),
            song_details: std::collections::BTreeMap::from([(
                "new-song".to_string(),
                ParsedSongDetail {
                    title: "New Song".to_string(),
                    genre: Some("Genre".to_string()),
                    artist: "Artist".to_string(),
                    chart_type: ChartType::Std,
                    difficulties: vec![ParsedSongChartDetail {
                        diff_category: DifficultyCategory::Basic,
                        level: "4".to_string(),
                        chart_type: ChartType::Std,
                        achievement_percent: Some(95.0),
                        rank: Some("AAA".parse().unwrap()),
                        fc: Some("FC".parse().unwrap()),
                        sync: Some("FS".parse().unwrap()),
                        dx_score: Some(950),
                        dx_score_max: Some(1000),
                        last_played_at: Some("2026/03/05 22:03".to_string()),
                        play_count: Some(2),
                    }],
                },
            )]),
        });
        let (_, score_updates) =
            resolve_recent_entries_and_collect_score_updates(&mut source, &filtered).await?;
        assert_eq!(score_updates.len(), 1);
        assert_eq!(source.fetch_log().len(), 2);

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
}
