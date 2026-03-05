use eyre::{Result, WrapErr};
use reqwest::Url;
use sqlx::SqlitePool;
use tracing::{info, warn};

use crate::db::{count_scores_rows, replace_scores};
use crate::http_client::MaimaiClient;
use crate::tasks::utils::auth::{ExpectedPage, fetch_html_with_auth_recovery};
use crate::tasks::utils::song_detail::{SongDetailCache, fetch_song_detail_by_idx};
use maimai_parsers::parse_scores_html;
use models::{ChartType, ParsedScoreEntry};

const MAX_SEED_DETAIL_RELOAD_RETRIES: usize = 5;

#[derive(Debug, Clone, Default)]
pub(crate) struct SeedScoresOutcome {
    pub(crate) seeded: bool,
    pub(crate) rows_written: usize,
}

#[derive(Debug, Clone)]
struct ScoreListSnapshot {
    entries: Vec<ParsedScoreEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SeedSongIndexEntry {
    ordinal: usize,
    title: String,
    chart_type: ChartType,
    basic_level: String,
    idx: String,
}

pub(crate) async fn ensure_scores_seeded(
    pool: &SqlitePool,
    client: &mut MaimaiClient,
) -> Result<SeedScoresOutcome> {
    let existing_rows = count_scores_rows(pool)
        .await
        .wrap_err("count scores rows")?;
    if existing_rows > 0 {
        return Ok(SeedScoresOutcome::default());
    }

    let seed_targets = fetch_seed_song_index_entries(client)
        .await
        .wrap_err("fetch diff=0 song index")?;
    let mut detail_cache = SongDetailCache::default();
    let mut entries = Vec::new();
    let total_songs = seed_targets.len();
    let started_at = std::time::Instant::now();

    info!("startup score seeding started: songs={total_songs}");

    for (idx, target) in seed_targets.iter().enumerate() {
        let detail = fetch_seed_song_detail(client, target, &mut detail_cache)
            .await
            .wrap_err_with(|| format!("fetch seed song detail for '{}'", target.title))?;
        entries.extend(score_entries_from_song_detail(detail));

        let processed = idx + 1;
        if should_log_seed_progress(processed, total_songs) {
            let percent = (processed as f64 / total_songs as f64) * 100.0;
            info!(
                "startup score seeding progress: songs={processed}/{total_songs} ({percent:.1}%) rows_collected={}",
                entries.len()
            );
        }
    }

    replace_scores(pool, &entries)
        .await
        .wrap_err("replace seeded scores rows")?;

    info!(
        "startup score seeding completed: songs={total_songs} rows_written={} elapsed_sec={:.1}",
        entries.len(),
        started_at.elapsed().as_secs_f64()
    );

    Ok(SeedScoresOutcome {
        seeded: true,
        rows_written: entries.len(),
    })
}

pub(crate) async fn backfill_missing_playlog_metadata(pool: &SqlitePool) -> Result<usize> {
    let rows = sqlx::query_as::<_, (i64, String, String, Option<String>)>(
        r#"
        SELECT played_at_unixtime, title, chart_type, diff_category
        FROM playlogs
        WHERE genre IS NULL OR artist IS NULL
        "#,
    )
    .fetch_all(pool)
    .await
    .wrap_err("fetch playlogs missing song metadata")?;

    let mut updated = 0usize;
    for (played_at_unixtime, title, chart_type, diff_category) in rows {
        let Some(diff_category) = diff_category else {
            continue;
        };

        let matches = sqlx::query_as::<_, (String, String)>(
            r#"
            SELECT DISTINCT genre, artist
            FROM scores
            WHERE title = ?1 AND chart_type = ?2 AND diff_category = ?3
            "#,
        )
        .bind(&title)
        .bind(&chart_type)
        .bind(&diff_category)
        .fetch_all(pool)
        .await
        .wrap_err("fetch candidate song metadata from scores")?;

        if matches.len() != 1 {
            continue;
        }

        let (genre, artist) = &matches[0];
        sqlx::query(
            r#"
            UPDATE playlogs
            SET genre = ?1, artist = ?2
            WHERE played_at_unixtime = ?3
            "#,
        )
        .bind(genre)
        .bind(artist)
        .bind(played_at_unixtime)
        .execute(pool)
        .await
        .wrap_err("update playlog metadata from scores")?;
        updated += 1;
    }

    Ok(updated)
}

async fn fetch_seed_song_index_entries(
    client: &mut MaimaiClient,
) -> Result<Vec<SeedSongIndexEntry>> {
    let snapshot = fetch_score_list_snapshot(client, 0)
        .await
        .wrap_err("fetch diff=0 snapshot")?;

    let mut targets = Vec::new();
    for (ordinal, entry) in snapshot.entries.into_iter().enumerate() {
        let Some(idx) = entry
            .source_idx
            .as_deref()
            .map(str::trim)
            .filter(|idx| !idx.is_empty())
            .map(str::to_string)
        else {
            continue;
        };

        targets.push(SeedSongIndexEntry {
            ordinal,
            title: entry.title,
            chart_type: entry.chart_type,
            basic_level: entry.level,
            idx,
        });
    }

    info!(
        "startup score seeding index loaded: songs={}",
        targets.len()
    );
    Ok(targets)
}

async fn fetch_score_list_snapshot(
    client: &mut MaimaiClient,
    diff: u8,
) -> Result<ScoreListSnapshot> {
    let url = scores_url(diff).wrap_err("build scores url")?;
    let html = fetch_html_with_auth_recovery(client, &url, ExpectedPage::ScoresList { diff })
        .await
        .wrap_err("fetch scores html with auth recovery")?;
    let entries = parse_scores_html(&html, diff).wrap_err("parse scores html")?;

    Ok(ScoreListSnapshot { entries })
}

async fn fetch_seed_song_detail(
    client: &mut MaimaiClient,
    target: &SeedSongIndexEntry,
    cache: &mut SongDetailCache,
) -> Result<models::ParsedSongDetail> {
    if let Some(detail) = cache.get(&target.idx) {
        return Ok(detail);
    }

    let mut current_target = target.clone();
    let mut last_err: Option<eyre::Report> = None;

    for attempt in 1..=MAX_SEED_DETAIL_RELOAD_RETRIES {
        if let Some(detail) = cache.get(&current_target.idx) {
            return Ok(detail);
        }

        match fetch_song_detail_by_idx(client, &current_target.idx).await {
            Ok(detail) => {
                cache.insert(current_target.idx.clone(), detail.clone());
                return Ok(detail);
            }
            Err(err) => {
                last_err = Some(err);
                if attempt == MAX_SEED_DETAIL_RELOAD_RETRIES {
                    break;
                }
                warn!(
                    "musicDetail fetch failed during startup seeding; reloading diff=0 snapshot: title='{}' ordinal={} idx={} attempt={}/{}",
                    current_target.title,
                    current_target.ordinal,
                    current_target.idx,
                    attempt,
                    MAX_SEED_DETAIL_RELOAD_RETRIES
                );

                let reloaded_targets = fetch_seed_song_index_entries(client)
                    .await
                    .wrap_err("reload diff=0 page after musicDetail failure")?;
                let reloaded = reloaded_targets.get(target.ordinal).ok_or_else(|| {
                    eyre::eyre!(
                        "reloaded diff=0 page has fewer songs than expected: ordinal={} total={}",
                        target.ordinal,
                        reloaded_targets.len()
                    )
                })?;

                if !seed_target_matches(target, reloaded) {
                    return Err(eyre::eyre!(
                        "diff=0 song ordering changed during startup retry at ordinal {}: expected ('{}', {}, '{}') but got ('{}', {}, '{}')",
                        target.ordinal,
                        target.title,
                        target.chart_type.as_str(),
                        target.basic_level,
                        reloaded.title,
                        reloaded.chart_type.as_str(),
                        reloaded.basic_level
                    ))
                    .wrap_err_with(|| {
                        format!(
                            "song identity mismatch while reloading diff=0 snapshot during startup seeding (ordinal={})",
                            target.ordinal
                        )
                    });
                }

                current_target = reloaded.clone();
            }
        }
    }

    Err(eyre::eyre!(
        "failed to fetch musicDetail during startup seeding after {} attempts: title='{}' ordinal={}",
        MAX_SEED_DETAIL_RELOAD_RETRIES,
        target.title,
        target.ordinal
    ))
    .wrap_err_with(|| {
        last_err
            .map(|e| e.to_string())
            .unwrap_or_else(|| "missing last error".to_string())
    })
}

fn should_log_seed_progress(processed: usize, total: usize) -> bool {
    processed == 1 || processed == total || processed.is_multiple_of(50)
}

fn seed_target_matches(expected: &SeedSongIndexEntry, actual: &SeedSongIndexEntry) -> bool {
    expected.title == actual.title
        && expected.chart_type == actual.chart_type
        && expected.basic_level == actual.basic_level
}

pub(crate) fn score_entries_from_song_detail(
    detail: models::ParsedSongDetail,
) -> Vec<ParsedScoreEntry> {
    let title = canonical_title_for_detail(&detail);
    let genre = detail.genre.clone().unwrap_or_default();
    let artist = detail.artist.clone();

    detail
        .difficulties
        .into_iter()
        .map(|difficulty| ParsedScoreEntry {
            title: title.clone(),
            genre: genre.clone(),
            artist: artist.clone(),
            chart_type: difficulty.chart_type,
            diff_category: difficulty.diff_category,
            level: difficulty.level,
            achievement_percent: difficulty.achievement_percent,
            rank: difficulty.rank,
            fc: difficulty.fc,
            sync: difficulty.sync,
            dx_score: difficulty.dx_score,
            dx_score_max: difficulty.dx_score_max,
            last_played_at: difficulty.last_played_at,
            play_count: difficulty.play_count,
            source_idx: None,
        })
        .collect()
}

pub(crate) fn canonical_title_for_detail(detail: &models::ParsedSongDetail) -> String {
    detail.title.trim().to_string()
}

fn scores_url(diff: u8) -> Result<Url> {
    if diff > 4 {
        return Err(eyre::eyre!("diff must be 0..4"));
    }

    Url::parse_with_params(
        "https://maimaidx-eng.com/maimai-mobile/record/musicGenre/search/",
        &[("genre", "99"), ("diff", &diff.to_string())],
    )
    .wrap_err("parse scores url")
}

#[cfg(test)]
mod tests {
    use super::*;
    use models::{DifficultyCategory, ParsedSongChartDetail, ParsedSongDetail};

    #[test]
    fn score_entries_from_song_detail_carries_song_identity() {
        let detail = ParsedSongDetail {
            title: "Song A".to_string(),
            genre: Some("Genre A".to_string()),
            artist: "Artist A".to_string(),
            chart_type: ChartType::Dx,
            difficulties: vec![ParsedSongChartDetail {
                diff_category: DifficultyCategory::Master,
                level: "12+".to_string(),
                chart_type: ChartType::Dx,
                achievement_percent: Some(100.5),
                rank: None,
                fc: None,
                sync: None,
                dx_score: Some(1200),
                dx_score_max: Some(1500),
                last_played_at: Some("2026/01/23 01:14".to_string()),
                play_count: Some(9),
            }],
        };

        let entries = score_entries_from_song_detail(detail);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].title, "Song A");
        assert_eq!(entries[0].genre, "Genre A");
        assert_eq!(entries[0].artist, "Artist A");
    }

    #[test]
    fn seed_target_match_requires_same_position_key() {
        let expected = SeedSongIndexEntry {
            ordinal: 10,
            title: "Song A".to_string(),
            chart_type: ChartType::Dx,
            basic_level: "3".to_string(),
            idx: "old".to_string(),
        };
        let reloaded_same = SeedSongIndexEntry {
            ordinal: 10,
            title: "Song A".to_string(),
            chart_type: ChartType::Dx,
            basic_level: "3".to_string(),
            idx: "new".to_string(),
        };
        let reloaded_other = SeedSongIndexEntry {
            ordinal: 10,
            title: "Song B".to_string(),
            chart_type: ChartType::Dx,
            basic_level: "3".to_string(),
            idx: "new".to_string(),
        };

        assert!(seed_target_matches(&expected, &reloaded_same));
        assert!(!seed_target_matches(&expected, &reloaded_other));
    }
}
