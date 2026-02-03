use std::str::FromStr;

use eyre::WrapErr;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{Pool, Sqlite};

use models::{ChartType, DifficultyCategory, ParsedPlayRecord, ParsedScoreEntry, ScoreEntry};

pub type SqlitePool = Pool<Sqlite>;

pub async fn connect(database_url: &str) -> eyre::Result<SqlitePool> {
    let options = SqliteConnectOptions::from_str(database_url)
        .wrap_err("parse database url")?
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
    sqlx::migrate!("../../record-collector-server/migrations")
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
        let Some(played_at_unixtime) = entry.played_at_unixtime else {
            continue;
        };
        insert_playlog(&mut tx, scraped_at, played_at_unixtime, entry).await?;
    }

    tx.commit().await.wrap_err("commit transaction")?;
    Ok(())
}

pub async fn clear_scores(pool: &SqlitePool) -> eyre::Result<()> {
    sqlx::query("DELETE FROM scores")
        .execute(pool)
        .await
        .wrap_err("clear scores")?;
    Ok(())
}

pub async fn clear_playlogs(pool: &SqlitePool) -> eyre::Result<()> {
    sqlx::query("DELETE FROM playlogs")
        .execute(pool)
        .await
        .wrap_err("clear playlogs")?;
    Ok(())
}

pub async fn clear_app_state(pool: &SqlitePool) -> eyre::Result<()> {
    sqlx::query("DELETE FROM app_state")
        .execute(pool)
        .await
        .wrap_err("clear app_state")?;
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

async fn upsert_score(
    tx: &mut sqlx::Transaction<'_, Sqlite>,
    scraped_at: i64,
    entry: &ParsedScoreEntry,
) -> eyre::Result<()> {
    let achievement_x10000 = percent_to_x10000(entry.achievement_percent);

    sqlx::query(
        r#"
		INSERT INTO scores (
		  title, chart_type, diff_category, level,
		  achievement_x10000, rank, fc, sync,
		  dx_score, dx_score_max,
		  source_idx, scraped_at
		)
		VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
		ON CONFLICT(title, chart_type, diff_category) DO UPDATE SET
		  level = excluded.level,
		  achievement_x10000 = excluded.achievement_x10000,
		  rank = excluded.rank,
		  fc = excluded.fc,
		  sync = excluded.sync,
		  dx_score = excluded.dx_score,
		  dx_score_max = excluded.dx_score_max,
		  source_idx = excluded.source_idx,
		  scraped_at = excluded.scraped_at
		"#,
    )
    .bind(&entry.title)
    .bind(chart_type_str(entry.chart_type))
    .bind(entry.diff_category.as_str())
    .bind(&entry.level)
    .bind(achievement_x10000)
    .bind(entry.rank.map(|r| r.as_str()))
    .bind(entry.fc.map(|v| v.as_str()))
    .bind(entry.sync.map(|v| v.as_str()))
    .bind(entry.dx_score)
    .bind(entry.dx_score_max)
    .bind(entry.source_idx.as_deref())
    .bind(scraped_at)
    .execute(&mut **tx)
    .await
    .wrap_err("upsert scores")?;
    Ok(())
}

async fn insert_playlog(
    tx: &mut sqlx::Transaction<'_, Sqlite>,
    scraped_at: i64,
    played_at_unixtime: i64,
    entry: &ParsedPlayRecord,
) -> eyre::Result<()> {
    let achievement_x10000 = percent_to_x10000(entry.achievement_percent);

    let achievement_new_record = i64::from(u8::from(entry.achievement_new_record));
    let first_play = i64::from(u8::from(entry.first_play));
    sqlx::query(
        r#"
	INSERT INTO playlogs (
	  played_at_unixtime,
	  played_at, track, credit_play_count,
	  title, chart_type, diff_category, level,
	  achievement_x10000, achievement_new_record, first_play,
	  score_rank, fc, sync,
	  dx_score, dx_score_max,
	  scraped_at
	)
	VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
	ON CONFLICT(played_at_unixtime) DO NOTHING
	"#,
    )
    .bind(played_at_unixtime)
    .bind(entry.played_at.as_deref())
    .bind(entry.track.map(i64::from))
    .bind(entry.credit_play_count.map(i64::from))
    .bind(&entry.title)
    .bind(chart_type_str(entry.chart_type))
    .bind(entry.diff_category.map(|d| d.as_str().to_string()))
    .bind(entry.level.as_deref())
    .bind(achievement_x10000)
    .bind(achievement_new_record)
    .bind(first_play)
    .bind(entry.score_rank.map(|r| r.as_str()))
    .bind(entry.fc.map(|v| v.as_str()))
    .bind(entry.sync.map(|v| v.as_str()))
    .bind(entry.dx_score)
    .bind(entry.dx_score_max)
    .bind(scraped_at)
    .execute(&mut **tx)
    .await
    .wrap_err("insert playlogs")?;
    Ok(())
}

fn chart_type_str(t: ChartType) -> &'static str {
    match t {
        ChartType::Std => "STD",
        ChartType::Dx => "DX",
    }
}

pub fn format_diff_category(diff_category: Option<DifficultyCategory>) -> &'static str {
    diff_category.map(|d| d.as_str()).unwrap_or("Unknown")
}

pub fn format_chart_type(chart_type: ChartType) -> &'static str {
    match chart_type {
        ChartType::Std => "STD",
        ChartType::Dx => "DX",
    }
}

pub fn format_percent_f32(percent: Option<f32>) -> String {
    percent
        .map(|p| format!("{:.4}%", p))
        .unwrap_or_else(|| "N/A".to_string())
}

pub fn format_percent_f64(percent: Option<f64>) -> String {
    percent
        .map(|p| format!("{:.4}%", p))
        .unwrap_or_else(|| "N/A".to_string())
}

fn percent_to_x10000(percent: Option<f32>) -> Option<i64> {
    percent.map(|p| (p as f64 * 10000.0).round() as i64)
}

pub fn format_track(track: Option<i64>) -> String {
    track
        .map(|t| format!("Track {}", t))
        .unwrap_or_else(|| "Single".to_string())
}

pub async fn query_score_by_pk(
    pool: &SqlitePool,
    title: &str,
    chart_type: &str,
    diff_category: &str,
) -> eyre::Result<Option<ScoreEntry>> {
    let row = sqlx::query_as::<_, ScoreEntry>(
        r#"
        SELECT
            title, chart_type, diff_category, level,
            achievement_x10000, rank, fc, sync,
            dx_score, dx_score_max, source_idx
        FROM scores
        WHERE title = ? AND chart_type = ? AND diff_category = ? AND achievement_x10000 IS NOT NULL
        "#,
    )
    .bind(title)
    .bind(chart_type)
    .bind(diff_category)
    .fetch_optional(pool)
    .await
    .wrap_err("query score by pk")?;

    Ok(row)
}
