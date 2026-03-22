use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use maistats_record_collector::db;
use maistats_record_collector::tasks::polling::cycle::run_cycle_with_source;
use maistats_record_collector::tasks::startup::startup_sync_with_source;
use maistats_record_collector::tasks::utils::recent::RecentSyncOutcome;
use maistats_record_collector::tasks::utils::source::{
    ExpectedPage, FixtureCollectorData, FixtureCollectorSource,
};
use models::{
    ChartType, DifficultyCategory, ParsedPlayRecord, ParsedPlayerProfile, ParsedSongChartDetail,
    ParsedSongDetail,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
struct ScoreSnapshot {
    title: String,
    genre: String,
    artist: String,
    chart_type: String,
    diff_category: String,
    achievement_x10000: Option<i64>,
    rank: Option<String>,
    fc: Option<String>,
    sync: Option<String>,
    dx_score: Option<i32>,
    dx_score_max: Option<i32>,
    last_played_at: Option<String>,
    play_count: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
struct PlaylogSnapshot {
    played_at_unixtime: i64,
    played_at: Option<String>,
    track: Option<i32>,
    title: String,
    genre: Option<String>,
    artist: Option<String>,
    chart_type: String,
    diff_category: Option<String>,
    achievement_x10000: Option<i64>,
    score_rank: Option<String>,
    fc: Option<String>,
    sync: Option<String>,
    dx_score: Option<i32>,
    dx_score_max: Option<i32>,
    credit_id: Option<i32>,
    achievement_new_record: Option<i32>,
}

fn fixture_dir(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/collector_sync")
        .join(name)
}

fn read_json<T: DeserializeOwned>(path: &Path) -> T {
    let raw = std::fs::read_to_string(path).expect("read fixture json");
    serde_json::from_str(&raw).expect("parse fixture json")
}

async fn test_db() -> eyre::Result<db::SqlitePool> {
    let pool = db::connect("sqlite::memory:").await?;
    db::migrate(&pool).await?;
    Ok(pool)
}

async fn snapshot_scores(pool: &db::SqlitePool) -> eyre::Result<Vec<ScoreSnapshot>> {
    sqlx::query_as::<_, ScoreSnapshot>(
        r#"
        SELECT title, genre, artist, chart_type, diff_category,
               achievement_x10000, rank, fc, sync,
               dx_score, dx_score_max, last_played_at, play_count
        FROM scores
        ORDER BY title, genre, artist, chart_type,
                 CASE diff_category
                   WHEN 'BASIC' THEN 0
                   WHEN 'ADVANCED' THEN 1
                   WHEN 'EXPERT' THEN 2
                   WHEN 'MASTER' THEN 3
                   WHEN 'Re:MASTER' THEN 4
                   ELSE 99
                 END
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

async fn snapshot_playlogs(pool: &db::SqlitePool) -> eyre::Result<Vec<PlaylogSnapshot>> {
    sqlx::query_as::<_, PlaylogSnapshot>(
        r#"
        SELECT played_at_unixtime, played_at, track,
               title, genre, artist, chart_type, diff_category,
               achievement_x10000, score_rank, fc, sync,
               dx_score, dx_score_max, credit_id, achievement_new_record
        FROM playlogs
        ORDER BY played_at_unixtime
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

async fn snapshot_app_state(pool: &db::SqlitePool) -> eyre::Result<BTreeMap<String, String>> {
    let rows =
        sqlx::query_as::<_, (String, String)>("SELECT key, value FROM app_state ORDER BY key")
            .fetch_all(pool)
            .await?;
    Ok(rows.into_iter().collect())
}

fn load_fixture_source(name: &str) -> FixtureCollectorSource {
    FixtureCollectorSource::from_fixture_dir(&fixture_dir(name)).expect("load fixture source")
}

fn assert_recent_outcome(
    outcome: &Option<RecentSyncOutcome>,
    expected_inserted_credits: usize,
    expected_inserted_playlogs: usize,
    expected_refreshed_scores: usize,
    expected_failed_targets: usize,
) {
    match outcome.as_ref().expect("recent outcome present") {
        RecentSyncOutcome::Updated {
            inserted_credits,
            inserted_playlogs,
            refreshed_scores,
            failed_targets,
        } => {
            assert_eq!(*inserted_credits, expected_inserted_credits);
            assert_eq!(*inserted_playlogs, expected_inserted_playlogs);
            assert_eq!(*refreshed_scores, expected_refreshed_scores);
            assert_eq!(*failed_targets, expected_failed_targets);
        }
        other => panic!("unexpected recent outcome: {other:?}"),
    }
}

fn build_full_recent_50_source() -> FixtureCollectorSource {
    let player_data = ParsedPlayerProfile {
        user_name: "fixture-user".to_string(),
        rating: 14_000,
        current_version_play_count: 120,
        total_play_count: 350,
    };

    let mut recent_entries = Vec::new();
    let mut played_at_unixtime = 350_000i64;

    for credit_offset in 0..12 {
        let hour = 20 - (credit_offset / 2);
        let minute_base: i32 = if credit_offset % 2 == 0 { 40 } else { 20 };
        let entries = [
            (
                "Song Foxtrot",
                "song-foxtrot",
                4,
                format!("2026/03/10 {hour:02}:{minute_base:02}"),
                ChartType::Dx,
                DifficultyCategory::Master,
                "13",
                99.7,
                true,
                Some("SSS"),
                Some("AP"),
                Some("FDX+"),
                2010,
                2100,
            ),
            (
                "Song Golf",
                "song-golf",
                3,
                format!("2026/03/10 {hour:02}:{:02}", minute_base.saturating_sub(10)),
                ChartType::Std,
                DifficultyCategory::Advanced,
                "8",
                97.0,
                false,
                Some("S"),
                Some("FC"),
                Some("FS"),
                980,
                1200,
            ),
            (
                "Song Hotel",
                "song-hotel",
                2,
                format!("2026/03/10 {hour:02}:{:02}", minute_base.saturating_sub(20)),
                ChartType::Std,
                DifficultyCategory::Basic,
                "4",
                88.8,
                false,
                Some("A"),
                None,
                None,
                800,
                1000,
            ),
            (
                "Song Foxtrot",
                "song-foxtrot",
                1,
                format!("2026/03/10 {hour:02}:{:02}", minute_base.saturating_sub(30)),
                ChartType::Dx,
                DifficultyCategory::Basic,
                "5",
                95.0,
                false,
                Some("AAA"),
                Some("FC"),
                Some("FS"),
                1500,
                1800,
            ),
        ];

        for (
            title,
            music_detail_idx,
            track,
            played_at,
            chart_type,
            diff_category,
            level,
            achievement_percent,
            achievement_new_record,
            score_rank,
            fc,
            sync,
            dx_score,
            dx_score_max,
        ) in entries
        {
            recent_entries.push(ParsedPlayRecord {
                played_at_unixtime: Some(played_at_unixtime),
                playlog_detail_idx: Some(format!("{music_detail_idx}::{played_at_unixtime}")),
                track: Some(track),
                played_at: Some(played_at),
                credit_id: None,
                title: title.to_string(),
                genre: None,
                artist: None,
                chart_type,
                diff_category: Some(diff_category),
                level: Some(level.to_string()),
                achievement_percent: Some(achievement_percent),
                achievement_new_record,
                score_rank: score_rank.and_then(|value| value.parse().ok()),
                fc: fc.and_then(|value| value.parse().ok()),
                sync: sync.and_then(|value| value.parse().ok()),
                dx_score: Some(dx_score),
                dx_score_max: Some(dx_score_max),
            });
            played_at_unixtime -= 1;
        }
    }

    for (title, music_detail_idx, track, minute) in [
        ("Song Golf", "song-golf", 4, 5),
        ("Song Hotel", "song-hotel", 3, 0),
    ] {
        recent_entries.push(ParsedPlayRecord {
            played_at_unixtime: Some(played_at_unixtime),
            playlog_detail_idx: Some(format!("{music_detail_idx}::{played_at_unixtime}")),
            track: Some(track),
            played_at: Some(format!("2026/03/09 18:{minute:02}")),
            credit_id: None,
            title: title.to_string(),
            genre: None,
            artist: None,
            chart_type: ChartType::Std,
            diff_category: Some(if title == "Song Golf" {
                DifficultyCategory::Advanced
            } else {
                DifficultyCategory::Basic
            }),
            level: Some(if title == "Song Golf" { "8" } else { "4" }.to_string()),
            achievement_percent: Some(if title == "Song Golf" { 96.0 } else { 87.0 }),
            achievement_new_record: false,
            score_rank: Some(if title == "Song Golf" {
                "S".parse().unwrap()
            } else {
                "A".parse().unwrap()
            }),
            fc: None,
            sync: None,
            dx_score: Some(if title == "Song Golf" { 970 } else { 790 }),
            dx_score_max: Some(if title == "Song Golf" { 1200 } else { 1000 }),
        });
        played_at_unixtime -= 1;
    }

    let song_details = BTreeMap::from([
        (
            "song-foxtrot".to_string(),
            ParsedSongDetail {
                title: "Song Foxtrot".to_string(),
                genre: Some("POPS".to_string()),
                artist: "Artist F".to_string(),
                chart_type: ChartType::Dx,
                difficulties: vec![
                    ParsedSongChartDetail {
                        diff_category: DifficultyCategory::Basic,
                        level: "5".to_string(),
                        chart_type: ChartType::Dx,
                        achievement_percent: Some(95.0),
                        rank: Some("AAA".parse().unwrap()),
                        fc: Some("FC".parse().unwrap()),
                        sync: Some("FS".parse().unwrap()),
                        dx_score: Some(1500),
                        dx_score_max: Some(1800),
                        last_played_at: Some("2026/03/10 20:10".to_string()),
                        play_count: Some(24),
                    },
                    ParsedSongChartDetail {
                        diff_category: DifficultyCategory::Master,
                        level: "13".to_string(),
                        chart_type: ChartType::Dx,
                        achievement_percent: Some(99.7),
                        rank: Some("SSS".parse().unwrap()),
                        fc: Some("AP".parse().unwrap()),
                        sync: Some("FDX+".parse().unwrap()),
                        dx_score: Some(2010),
                        dx_score_max: Some(2100),
                        last_played_at: Some("2026/03/10 20:40".to_string()),
                        play_count: Some(12),
                    },
                ],
            },
        ),
        (
            "song-golf".to_string(),
            ParsedSongDetail {
                title: "Song Golf".to_string(),
                genre: Some("GAME".to_string()),
                artist: "Artist G".to_string(),
                chart_type: ChartType::Std,
                difficulties: vec![ParsedSongChartDetail {
                    diff_category: DifficultyCategory::Advanced,
                    level: "8".to_string(),
                    chart_type: ChartType::Std,
                    achievement_percent: Some(97.0),
                    rank: Some("S".parse().unwrap()),
                    fc: Some("FC".parse().unwrap()),
                    sync: Some("FS".parse().unwrap()),
                    dx_score: Some(980),
                    dx_score_max: Some(1200),
                    last_played_at: Some("2026/03/10 20:30".to_string()),
                    play_count: Some(18),
                }],
            },
        ),
        (
            "song-hotel".to_string(),
            ParsedSongDetail {
                title: "Song Hotel".to_string(),
                genre: Some("VARIETY".to_string()),
                artist: "Artist H".to_string(),
                chart_type: ChartType::Std,
                difficulties: vec![ParsedSongChartDetail {
                    diff_category: DifficultyCategory::Basic,
                    level: "4".to_string(),
                    chart_type: ChartType::Std,
                    achievement_percent: Some(88.8),
                    rank: Some("A".parse().unwrap()),
                    fc: None,
                    sync: None,
                    dx_score: Some(800),
                    dx_score_max: Some(1000),
                    last_played_at: Some("2026/03/10 20:20".to_string()),
                    play_count: Some(18),
                }],
            },
        ),
    ]);

    FixtureCollectorSource::from_data(FixtureCollectorData {
        player_data: Some(player_data),
        recent_entries: Some(recent_entries),
        score_lists: Default::default(),
        playlog_details: BTreeMap::new(),
        song_details,
    })
}

#[tokio::test]
async fn seed_small_startup_matches_expected_snapshots() -> eyre::Result<()> {
    let pool = test_db().await?;
    let mut source = load_fixture_source("seed_small_startup");

    let report = startup_sync_with_source(&pool, &mut source).await?;

    assert!(!report.skipped_for_maintenance);
    assert!(report.seeded);
    assert_eq!(report.seeded_rows_written, 9);
    assert_recent_outcome(&report.recent_outcome, 1, 3, 6, 0);

    let expected_scores: Vec<ScoreSnapshot> =
        read_json(&fixture_dir("seed_small_startup").join("expected_scores.json"));
    let expected_playlogs: Vec<PlaylogSnapshot> =
        read_json(&fixture_dir("seed_small_startup").join("expected_playlogs.json"));
    let expected_app_state: BTreeMap<String, String> =
        read_json(&fixture_dir("seed_small_startup").join("expected_app_state.json"));

    assert_eq!(snapshot_scores(&pool).await?, expected_scores);
    assert_eq!(snapshot_playlogs(&pool).await?, expected_playlogs);
    assert_eq!(snapshot_app_state(&pool).await?, expected_app_state);

    Ok(())
}

#[tokio::test]
async fn polling_update_small_updates_scores_and_playlogs() -> eyre::Result<()> {
    let pool = test_db().await?;
    let mut seed_source = load_fixture_source("seed_small_startup");
    startup_sync_with_source(&pool, &mut seed_source).await?;

    let mut update_source = load_fixture_source("polling_update_small");
    let report = run_cycle_with_source(&pool, &mut update_source).await?;

    assert!(!report.skipped_for_maintenance);
    assert!(!report.seeded);
    assert_recent_outcome(&report.recent_outcome, 1, 2, 2, 0);

    let expected_scores: Vec<ScoreSnapshot> =
        read_json(&fixture_dir("polling_update_small").join("expected_scores.json"));
    let expected_playlogs: Vec<PlaylogSnapshot> =
        read_json(&fixture_dir("polling_update_small").join("expected_playlogs.json"));
    let expected_app_state: BTreeMap<String, String> =
        read_json(&fixture_dir("polling_update_small").join("expected_app_state.json"));

    assert_eq!(snapshot_scores(&pool).await?, expected_scores);
    assert_eq!(snapshot_playlogs(&pool).await?, expected_playlogs);
    assert_eq!(snapshot_app_state(&pool).await?, expected_app_state);

    Ok(())
}

#[tokio::test]
async fn polling_partial_resolve_keeps_playlogs() -> eyre::Result<()> {
    let pool = test_db().await?;
    let mut seed_source = load_fixture_source("seed_small_startup");
    startup_sync_with_source(&pool, &mut seed_source).await?;

    let mut update_source = load_fixture_source("polling_partial_resolve_keeps_playlogs");
    let report = run_cycle_with_source(&pool, &mut update_source).await?;

    assert!(!report.skipped_for_maintenance);
    assert!(!report.seeded);
    assert_recent_outcome(&report.recent_outcome, 1, 2, 2, 1);

    let expected_scores: Vec<ScoreSnapshot> = read_json(
        &fixture_dir("polling_partial_resolve_keeps_playlogs").join("expected_scores.json"),
    );
    let expected_playlogs: Vec<PlaylogSnapshot> = read_json(
        &fixture_dir("polling_partial_resolve_keeps_playlogs").join("expected_playlogs.json"),
    );
    let expected_app_state: BTreeMap<String, String> = read_json(
        &fixture_dir("polling_partial_resolve_keeps_playlogs").join("expected_app_state.json"),
    );

    assert_eq!(snapshot_scores(&pool).await?, expected_scores);
    assert_eq!(snapshot_playlogs(&pool).await?, expected_playlogs);
    assert_eq!(snapshot_app_state(&pool).await?, expected_app_state);

    Ok(())
}

#[tokio::test]
async fn polling_unresolved_only_keeps_playlogs() -> eyre::Result<()> {
    let pool = test_db().await?;
    let mut seed_source = load_fixture_source("seed_small_startup");
    startup_sync_with_source(&pool, &mut seed_source).await?;

    let mut update_source = load_fixture_source("polling_unresolved_only_keeps_playlogs");
    let report = run_cycle_with_source(&pool, &mut update_source).await?;

    assert!(!report.skipped_for_maintenance);
    assert!(!report.seeded);
    assert_recent_outcome(&report.recent_outcome, 1, 1, 0, 1);

    let expected_scores: Vec<ScoreSnapshot> = read_json(
        &fixture_dir("polling_unresolved_only_keeps_playlogs").join("expected_scores.json"),
    );
    let expected_playlogs: Vec<PlaylogSnapshot> = read_json(
        &fixture_dir("polling_unresolved_only_keeps_playlogs").join("expected_playlogs.json"),
    );
    let expected_app_state: BTreeMap<String, String> = read_json(
        &fixture_dir("polling_unresolved_only_keeps_playlogs").join("expected_app_state.json"),
    );

    assert_eq!(snapshot_scores(&pool).await?, expected_scores);
    assert_eq!(snapshot_playlogs(&pool).await?, expected_playlogs);
    assert_eq!(snapshot_app_state(&pool).await?, expected_app_state);

    Ok(())
}

#[tokio::test]
async fn polling_unchanged_skips_recent_fetches() -> eyre::Result<()> {
    let pool = test_db().await?;
    let mut seed_source = load_fixture_source("seed_small_startup");
    startup_sync_with_source(&pool, &mut seed_source).await?;

    let initial_playlog_count = snapshot_playlogs(&pool).await?.len();
    let initial_score_count = snapshot_scores(&pool).await?.len();

    let mut unchanged_source = load_fixture_source("polling_unchanged_skips_recent");
    let report = run_cycle_with_source(&pool, &mut unchanged_source).await?;

    assert!(!report.skipped_for_maintenance);
    assert!(matches!(
        report.recent_outcome,
        Some(RecentSyncOutcome::SkippedUnchanged)
    ));
    assert_eq!(unchanged_source.fetch_log(), &[ExpectedPage::PlayerData]);
    assert_eq!(snapshot_playlogs(&pool).await?.len(), initial_playlog_count);
    assert_eq!(snapshot_scores(&pool).await?.len(), initial_score_count);

    Ok(())
}

#[tokio::test]
async fn polling_unchanged_backfills_incomplete_player_snapshot_only() -> eyre::Result<()> {
    let pool = test_db().await?;
    let mut seed_source = load_fixture_source("seed_small_startup");
    startup_sync_with_source(&pool, &mut seed_source).await?;

    sqlx::query("DELETE FROM app_state WHERE key = ?")
        .bind("player.current_version_play_count")
        .execute(&pool)
        .await?;

    let initial_playlog_count = snapshot_playlogs(&pool).await?.len();
    let initial_score_count = snapshot_scores(&pool).await?.len();

    let mut unchanged_source = load_fixture_source("polling_unchanged_skips_recent");
    let report = run_cycle_with_source(&pool, &mut unchanged_source).await?;

    assert!(!report.skipped_for_maintenance);
    assert!(matches!(
        report.recent_outcome,
        Some(RecentSyncOutcome::SkippedUnchanged)
    ));
    assert_eq!(unchanged_source.fetch_log(), &[ExpectedPage::PlayerData]);
    assert_eq!(snapshot_playlogs(&pool).await?.len(), initial_playlog_count);
    assert_eq!(snapshot_scores(&pool).await?.len(), initial_score_count);

    let app_state = snapshot_app_state(&pool).await?;
    assert_eq!(
        app_state
            .get("player.current_version_play_count")
            .map(String::as_str),
        Some("50")
    );

    Ok(())
}

#[tokio::test]
async fn polling_full_recent_truncates_oldest_partial_credit() -> eyre::Result<()> {
    let pool = test_db().await?;
    let mut source = build_full_recent_50_source();

    let report = run_cycle_with_source(&pool, &mut source).await?;

    assert!(!report.skipped_for_maintenance);
    assert_recent_outcome(&report.recent_outcome, 12, 48, 4, 0);

    let playlogs = snapshot_playlogs(&pool).await?;
    assert_eq!(playlogs.len(), 48);
    assert_eq!(
        playlogs.first().and_then(|entry| entry.credit_id),
        Some(339)
    );
    assert_eq!(playlogs.last().and_then(|entry| entry.credit_id), Some(350));

    let distinct_credit_ids = playlogs
        .iter()
        .filter_map(|entry| entry.credit_id)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(distinct_credit_ids.len(), 12);
    assert_eq!(distinct_credit_ids.first().copied(), Some(339));
    assert_eq!(distinct_credit_ids.last().copied(), Some(350));

    let scores = snapshot_scores(&pool).await?;
    assert_eq!(scores.len(), 4);
    assert!(scores.iter().any(|score| {
        score.title == "Song Foxtrot"
            && score.diff_category == "MASTER"
            && score.last_played_at.as_deref() == Some("2026/03/10 20:40")
            && score.play_count == Some(12)
    }));
    assert!(scores.iter().any(|score| {
        score.title == "Song Golf"
            && score.diff_category == "ADVANCED"
            && score.last_played_at.as_deref() == Some("2026/03/10 20:30")
    }));
    assert!(scores.iter().any(|score| {
        score.title == "Song Hotel"
            && score.diff_category == "BASIC"
            && score.last_played_at.as_deref() == Some("2026/03/10 20:20")
    }));

    let mut rerun_source = build_full_recent_50_source();
    let rerun = run_cycle_with_source(&pool, &mut rerun_source).await?;
    assert!(matches!(
        rerun.recent_outcome,
        Some(RecentSyncOutcome::SkippedUnchanged)
    ));
    assert_eq!(snapshot_playlogs(&pool).await?.len(), 48);

    Ok(())
}
