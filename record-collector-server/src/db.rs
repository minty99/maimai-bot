use std::str::FromStr;

use eyre::WrapErr;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{Pool, Sqlite};

use models::{ChartType, ParsedPlayRecord, ParsedScoreEntry};

pub(crate) type SqlitePool = Pool<Sqlite>;

pub(crate) async fn connect(database_url: &str) -> eyre::Result<SqlitePool> {
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

pub(crate) async fn migrate(pool: &SqlitePool) -> eyre::Result<()> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .wrap_err("run migrations")?;
    Ok(())
}

pub(crate) async fn upsert_scores(
    pool: &SqlitePool,
    entries: &[ParsedScoreEntry],
) -> eyre::Result<()> {
    let mut tx = pool.begin().await.wrap_err("begin transaction")?;

    for entry in entries {
        upsert_score(&mut tx, entry).await?;
    }

    tx.commit().await.wrap_err("commit transaction")?;
    Ok(())
}

pub(crate) async fn replace_scores(
    pool: &SqlitePool,
    entries: &[ParsedScoreEntry],
) -> eyre::Result<()> {
    let mut tx = pool.begin().await.wrap_err("begin transaction")?;

    sqlx::query("DELETE FROM scores")
        .execute(&mut *tx)
        .await
        .wrap_err("clear scores before replace")?;

    for entry in entries {
        upsert_score(&mut tx, entry).await?;
    }

    tx.commit().await.wrap_err("commit transaction")?;
    Ok(())
}

pub(crate) async fn upsert_playlogs(
    pool: &SqlitePool,
    entries: &[ParsedPlayRecord],
) -> eyre::Result<()> {
    let mut tx = pool.begin().await.wrap_err("begin transaction")?;

    for entry in entries {
        let Some(played_at_unixtime) = entry.played_at_unixtime else {
            continue;
        };
        insert_playlog(&mut tx, played_at_unixtime, entry).await?;
    }

    tx.commit().await.wrap_err("commit transaction")?;
    Ok(())
}
pub(crate) async fn count_scores_rows(pool: &SqlitePool) -> eyre::Result<i64> {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM scores")
        .fetch_one(pool)
        .await
        .wrap_err("count scores rows")
}

pub(crate) async fn get_app_state_u32(pool: &SqlitePool, key: &str) -> eyre::Result<Option<u32>> {
    let value = sqlx::query_scalar::<_, String>("SELECT value FROM app_state WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await
        .wrap_err("get app_state value")?;
    let Some(value) = value else {
        return Ok(None);
    };
    let parsed = value.parse::<u32>().wrap_err("parse app_state as u32")?;
    Ok(Some(parsed))
}

pub(crate) async fn set_app_state_u32(
    pool: &SqlitePool,
    key: &str,
    value: u32,
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
    .bind(value.to_string())
    .bind(updated_at)
    .execute(pool)
    .await
    .wrap_err("set app_state value")?;
    Ok(())
}

async fn upsert_score(
    tx: &mut sqlx::Transaction<'_, Sqlite>,
    entry: &ParsedScoreEntry,
) -> eyre::Result<()> {
    let achievement_x10000 = percent_to_x10000(entry.achievement_percent);

    sqlx::query(
        r#"
		INSERT INTO scores (
		  title, genre, artist, chart_type, diff_category,
		  achievement_x10000, rank, fc, sync,
		  dx_score, dx_score_max, last_played_at, play_count
		)
		VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
		ON CONFLICT(title, chart_type, diff_category, genre, artist) DO UPDATE SET
		  achievement_x10000 = excluded.achievement_x10000,
		  rank = excluded.rank,
		  fc = excluded.fc,
		  sync = excluded.sync,
		  dx_score = excluded.dx_score,
		  dx_score_max = excluded.dx_score_max,
		  last_played_at = excluded.last_played_at,
		  play_count = excluded.play_count
		"#,
    )
    .bind(&entry.title)
    .bind(&entry.genre)
    .bind(&entry.artist)
    .bind(chart_type_str(entry.chart_type))
    .bind(entry.diff_category.as_str())
    .bind(achievement_x10000)
    .bind(entry.rank.map(|r| r.as_str()))
    .bind(entry.fc.map(|v| v.as_str()))
    .bind(entry.sync.map(|v| v.as_str()))
    .bind(entry.dx_score)
    .bind(entry.dx_score_max)
    .bind(entry.last_played_at.as_deref())
    .bind(entry.play_count.map(i64::from))
    .execute(&mut **tx)
    .await
    .wrap_err("upsert scores")?;
    Ok(())
}

async fn insert_playlog(
    tx: &mut sqlx::Transaction<'_, Sqlite>,
    played_at_unixtime: i64,
    entry: &ParsedPlayRecord,
) -> eyre::Result<()> {
    let achievement_x10000 = percent_to_x10000(entry.achievement_percent);

    let achievement_new_record = i64::from(u8::from(entry.achievement_new_record));
    sqlx::query(
        r#"
	INSERT INTO playlogs (
	  played_at_unixtime,
	  played_at, track, credit_id,
	  title, genre, artist, chart_type, diff_category,
	  achievement_x10000, achievement_new_record,
	  score_rank, fc, sync,
	  dx_score, dx_score_max
	)
	VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
	ON CONFLICT(played_at_unixtime) DO NOTHING
	"#,
    )
    .bind(played_at_unixtime)
    .bind(entry.played_at.as_deref())
    .bind(entry.track.map(i64::from))
    .bind(entry.credit_id.map(i64::from))
    .bind(&entry.title)
    .bind(entry.genre.as_deref())
    .bind(entry.artist.as_deref())
    .bind(chart_type_str(entry.chart_type))
    .bind(entry.diff_category.map(|d| d.as_str().to_string()))
    .bind(achievement_x10000)
    .bind(achievement_new_record)
    .bind(entry.score_rank.map(|r| r.as_str()))
    .bind(entry.fc.map(|v| v.as_str()))
    .bind(entry.sync.map(|v| v.as_str()))
    .bind(entry.dx_score)
    .bind(entry.dx_score_max)
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

fn percent_to_x10000(percent: Option<f32>) -> Option<i64> {
    percent.map(|p| (p as f64 * 10000.0).round() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use models::DifficultyCategory;

    #[tokio::test]
    async fn upsert_scores_overwrites_detail_fields() -> eyre::Result<()> {
        let pool = connect("sqlite::memory:").await?;
        migrate(&pool).await?;

        let first = ParsedScoreEntry {
            title: "Song A".to_string(),
            genre: "Genre A".to_string(),
            artist: "Artist A".to_string(),
            chart_type: ChartType::Dx,
            diff_category: DifficultyCategory::Master,
            level: "12+".to_string(),
            achievement_percent: Some(99.1234),
            rank: None,
            fc: None,
            sync: None,
            dx_score: Some(1000),
            dx_score_max: Some(2000),
            last_played_at: Some("2026/01/20 00:00".to_string()),
            play_count: Some(3),
            source_idx: None,
        };
        upsert_scores(&pool, &[first]).await?;

        let second = ParsedScoreEntry {
            title: "Song A".to_string(),
            genre: "Genre A".to_string(),
            artist: "Artist A".to_string(),
            chart_type: ChartType::Dx,
            diff_category: DifficultyCategory::Master,
            level: "12+".to_string(),
            achievement_percent: Some(100.5),
            rank: None,
            fc: None,
            sync: None,
            dx_score: Some(1234),
            dx_score_max: Some(2345),
            last_played_at: Some("2026/01/23 01:14".to_string()),
            play_count: Some(7),
            source_idx: None,
        };
        upsert_scores(&pool, &[second]).await?;

        #[expect(clippy::type_complexity)]
        let row: (
            Option<i64>,
            Option<i32>,
            Option<i32>,
            Option<String>,
            Option<i64>,
        ) = sqlx::query_as(
            r#"
                SELECT achievement_x10000, dx_score, dx_score_max, last_played_at, play_count
                FROM scores
                WHERE title = 'Song A' AND genre = 'Genre A' AND artist = 'Artist A' AND chart_type = 'DX' AND diff_category = 'MASTER'
                "#,
        )
        .fetch_one(&pool)
        .await?;

        assert_eq!(row.0, Some(1_005_000));
        assert_eq!(row.1, Some(1234));
        assert_eq!(row.2, Some(2345));
        assert_eq!(row.3.as_deref(), Some("2026/01/23 01:14"));
        assert_eq!(row.4, Some(7));

        Ok(())
    }

    #[tokio::test]
    async fn replace_scores_replaces_existing_rows_transactionally() -> eyre::Result<()> {
        let pool = connect("sqlite::memory:").await?;
        migrate(&pool).await?;

        let initial = ParsedScoreEntry {
            title: "Song A".to_string(),
            genre: "Genre A".to_string(),
            artist: "Artist A".to_string(),
            chart_type: ChartType::Dx,
            diff_category: DifficultyCategory::Master,
            level: "12+".to_string(),
            achievement_percent: Some(99.1234),
            rank: None,
            fc: None,
            sync: None,
            dx_score: Some(1000),
            dx_score_max: Some(2000),
            last_played_at: Some("2026/01/20 00:00".to_string()),
            play_count: Some(3),
            source_idx: None,
        };
        upsert_scores(&pool, &[initial]).await?;

        let replacement = ParsedScoreEntry {
            title: "Song B".to_string(),
            genre: "Genre B".to_string(),
            artist: "Artist B".to_string(),
            chart_type: ChartType::Std,
            diff_category: DifficultyCategory::Expert,
            level: "11+".to_string(),
            achievement_percent: Some(98.0),
            rank: None,
            fc: None,
            sync: None,
            dx_score: Some(900),
            dx_score_max: Some(1900),
            last_played_at: None,
            play_count: None,
            source_idx: None,
        };
        replace_scores(&pool, &[replacement]).await?;

        let titles: Vec<String> = sqlx::query_scalar("SELECT title FROM scores ORDER BY title")
            .fetch_all(&pool)
            .await?;
        assert_eq!(titles, vec!["Song B".to_string()]);

        Ok(())
    }

    #[tokio::test]
    async fn get_app_state_u32_returns_none_for_missing_key() -> eyre::Result<()> {
        let pool = connect("sqlite::memory:").await?;
        migrate(&pool).await?;

        assert_eq!(get_app_state_u32(&pool, "missing").await?, None);

        Ok(())
    }
}
