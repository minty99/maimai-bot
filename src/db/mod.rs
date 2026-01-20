use std::path::Path;

use eyre::WrapErr;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{Pool, Sqlite};

use crate::maimai::models::{ChartType, ParsedPlayRecord, ParsedScoreEntry};

pub type SqlitePool = Pool<Sqlite>;

pub async fn connect(db_path: &Path) -> eyre::Result<SqlitePool> {
    let options = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal);

    SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .wrap_err("connect sqlite")
}

pub async fn migrate(pool: &SqlitePool) -> eyre::Result<()> {
    sqlx::migrate!()
        .run(pool)
        .await
        .wrap_err("run migrations")?;
    Ok(())
}

pub async fn upsert_scores(
    pool: &SqlitePool,
    scraped_at: i64,
    entries: &[ParsedScoreEntry],
) -> eyre::Result<()> {
    let mut tx = pool.begin().await.wrap_err("begin transaction")?;

    for entry in entries {
        upsert_song(&mut tx, &entry.song_key, &entry.title, scraped_at).await?;
        upsert_score(&mut tx, scraped_at, entry).await?;
    }

    tx.commit().await.wrap_err("commit transaction")?;
    Ok(())
}

pub async fn upsert_playlogs(
    pool: &SqlitePool,
    scraped_at: i64,
    entries: &[ParsedPlayRecord],
) -> eyre::Result<()> {
    let mut tx = pool.begin().await.wrap_err("begin transaction")?;

    for entry in entries {
        let Some(playlog_idx) = entry.playlog_idx.as_deref() else {
            continue;
        };
        upsert_song(&mut tx, &entry.song_key, &entry.title, scraped_at).await?;
        upsert_playlog(&mut tx, scraped_at, playlog_idx, entry).await?;
    }

    tx.commit().await.wrap_err("commit transaction")?;
    Ok(())
}

pub async fn get_app_state(pool: &SqlitePool, key: &str) -> eyre::Result<Option<String>> {
    sqlx::query_scalar::<_, Option<String>>("SELECT value FROM app_state WHERE key = ?")
        .bind(key)
        .fetch_one(pool)
        .await
        .wrap_err("get app_state value")
}

pub async fn set_app_state(
    pool: &SqlitePool,
    key: &str,
    value: &str,
    updated_at: i64,
) -> eyre::Result<()> {
    sqlx::query(
        r#"
INSERT INTO app_state (key, value, updated_at)
VALUES (?1, ?2, ?3)
ON CONFLICT(key) DO UPDATE SET
  value = excluded.value,
  updated_at = excluded.updated_at
"#,
    )
    .bind(key)
    .bind(value)
    .bind(updated_at)
    .execute(pool)
    .await
    .wrap_err("set app_state value")?;
    Ok(())
}

pub async fn get_app_state_u32(pool: &SqlitePool, key: &str) -> eyre::Result<Option<u32>> {
    let Some(value) = get_app_state(pool, key).await? else {
        return Ok(None);
    };
    let parsed = value.parse::<u32>().wrap_err("parse app_state as u32")?;
    Ok(Some(parsed))
}

pub async fn set_app_state_u32(
    pool: &SqlitePool,
    key: &str,
    value: u32,
    updated_at: i64,
) -> eyre::Result<()> {
    set_app_state(pool, key, &value.to_string(), updated_at).await
}

async fn upsert_song(
    tx: &mut sqlx::Transaction<'_, Sqlite>,
    song_key: &str,
    title: &str,
    now: i64,
) -> eyre::Result<()> {
    sqlx::query(
        r#"
INSERT INTO songs (song_key, title, created_at, updated_at)
VALUES (?1, ?2, ?3, ?3)
ON CONFLICT(song_key) DO UPDATE SET
  title = excluded.title,
  updated_at = excluded.updated_at
"#,
    )
    .bind(song_key)
    .bind(title)
    .bind(now)
    .execute(&mut **tx)
    .await
    .wrap_err("upsert songs")?;
    Ok(())
}

async fn upsert_score(
    tx: &mut sqlx::Transaction<'_, Sqlite>,
    scraped_at: i64,
    entry: &ParsedScoreEntry,
) -> eyre::Result<()> {
    sqlx::query(
        r#"
INSERT INTO scores (
  song_key, chart_type, diff,
  achievement_percent, rank, fc, sync,
  dx_score, dx_score_max,
  source_idx, scraped_at
)
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
ON CONFLICT(song_key, chart_type, diff) DO UPDATE SET
  achievement_percent = excluded.achievement_percent,
  rank = excluded.rank,
  fc = excluded.fc,
  sync = excluded.sync,
  dx_score = excluded.dx_score,
  dx_score_max = excluded.dx_score_max,
  source_idx = excluded.source_idx,
  scraped_at = excluded.scraped_at
"#,
    )
    .bind(&entry.song_key)
    .bind(chart_type_str(entry.chart_type))
    .bind(i64::from(entry.diff))
    .bind(entry.achievement_percent.map(f64::from))
    .bind(entry.rank.as_deref())
    .bind(entry.fc.as_deref())
    .bind(entry.sync.as_deref())
    .bind(entry.dx_score)
    .bind(entry.dx_score_max)
    .bind(entry.source_idx.as_deref())
    .bind(scraped_at)
    .execute(&mut **tx)
    .await
    .wrap_err("upsert scores")?;
    Ok(())
}

async fn upsert_playlog(
    tx: &mut sqlx::Transaction<'_, Sqlite>,
    scraped_at: i64,
    playlog_idx: &str,
    entry: &ParsedPlayRecord,
) -> eyre::Result<()> {
    sqlx::query(
        r#"
INSERT INTO playlogs (
  playlog_idx,
  played_at, track,
  song_key, title, chart_type, diff,
  achievement_percent, score_rank, fc, sync,
  dx_score, dx_score_max,
  scraped_at
)
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
ON CONFLICT(playlog_idx) DO UPDATE SET
  played_at = excluded.played_at,
  track = excluded.track,
  song_key = excluded.song_key,
  title = excluded.title,
  chart_type = excluded.chart_type,
  diff = excluded.diff,
  achievement_percent = excluded.achievement_percent,
  score_rank = excluded.score_rank,
  fc = excluded.fc,
  sync = excluded.sync,
  dx_score = excluded.dx_score,
  dx_score_max = excluded.dx_score_max,
  scraped_at = excluded.scraped_at
"#,
    )
    .bind(playlog_idx)
    .bind(entry.played_at.as_deref())
    .bind(entry.track.map(i64::from))
    .bind(&entry.song_key)
    .bind(&entry.title)
    .bind(chart_type_str(entry.chart_type))
    .bind(entry.diff.map(i64::from))
    .bind(entry.achievement_percent.map(f64::from))
    .bind(entry.score_rank.as_deref())
    .bind(entry.fc.as_deref())
    .bind(entry.sync.as_deref())
    .bind(entry.dx_score)
    .bind(entry.dx_score_max)
    .bind(scraped_at)
    .execute(&mut **tx)
    .await
    .wrap_err("upsert playlogs")?;
    Ok(())
}

fn chart_type_str(t: ChartType) -> &'static str {
    match t {
        ChartType::Std => "STD",
        ChartType::Dx => "DX",
    }
}

pub fn format_diff(diff: Option<u8>) -> &'static str {
    match diff {
        Some(0) => "BASIC",
        Some(1) => "ADVANCED",
        Some(2) => "EXPERT",
        Some(3) => "MASTER",
        Some(4) => "Re:MASTER",
        _ => "Unknown",
    }
}

pub fn format_chart_type(chart_type: ChartType) -> &'static str {
    match chart_type {
        ChartType::Std => "STD",
        ChartType::Dx => "DX",
    }
}

pub fn format_percent_f32(percent: Option<f32>) -> String {
    percent
        .map(|p| format!("{:.2}%", p))
        .unwrap_or_else(|| "N/A".to_string())
}

pub fn format_percent_f64(percent: Option<f64>) -> String {
    percent
        .map(|p| format!("{:.2}%", p))
        .unwrap_or_else(|| "N/A".to_string())
}

pub fn format_track(track: Option<i64>) -> String {
    track
        .map(|t| format!("Track {}", t))
        .unwrap_or_else(|| "Single".to_string())
}
