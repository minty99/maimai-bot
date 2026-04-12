use std::str::FromStr;

use eyre::WrapErr;
use poise::serenity_prelude as serenity;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{Pool, Sqlite};

pub(crate) type SqlitePool = Pool<Sqlite>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Registration {
    pub(crate) discord_user_id: serenity::UserId,
    pub(crate) record_collector_server_url: String,
}

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

pub(crate) async fn upsert_registration(
    pool: &SqlitePool,
    discord_user_id: serenity::UserId,
    record_collector_server_url: &str,
    now_unix: i64,
) -> eyre::Result<()> {
    sqlx::query(
        r#"
INSERT INTO discord_user_record_collectors (
  discord_user_id,
  record_collector_server_url,
  created_at,
  updated_at
)
VALUES (?1, ?2, ?3, ?4)
ON CONFLICT(discord_user_id) DO UPDATE SET
  record_collector_server_url = excluded.record_collector_server_url,
  updated_at = excluded.updated_at
"#,
    )
    .bind(discord_user_id.to_string())
    .bind(record_collector_server_url)
    .bind(now_unix)
    .bind(now_unix)
    .execute(pool)
    .await
    .wrap_err("upsert registration")?;
    Ok(())
}

pub(crate) async fn get_registration(
    pool: &SqlitePool,
    discord_user_id: serenity::UserId,
) -> eyre::Result<Option<Registration>> {
    let row = sqlx::query_as::<_, (String, String)>(
        r#"
SELECT discord_user_id, record_collector_server_url
FROM discord_user_record_collectors
WHERE discord_user_id = ?1
"#,
    )
    .bind(discord_user_id.to_string())
    .fetch_optional(pool)
    .await
    .wrap_err("fetch registration")?;

    let Some((discord_user_id, record_collector_server_url)) = row else {
        return Ok(None);
    };

    let parsed_id = discord_user_id
        .parse::<u64>()
        .wrap_err("parse discord_user_id from database")?;

    Ok(Some(Registration {
        discord_user_id: serenity::UserId::new(parsed_id),
        record_collector_server_url,
    }))
}

pub(crate) async fn count_registrations(pool: &SqlitePool) -> eyre::Result<i64> {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM discord_user_record_collectors")
        .fetch_one(pool)
        .await
        .wrap_err("count registrations")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PersistedUpdownSession {
    pub(crate) discord_user_id: serenity::UserId,
    pub(crate) thread_channel_id: serenity::ChannelId,
    pub(crate) pick_message_id: serenity::MessageId,
    pub(crate) current_level_tenths: i16,
}

pub(crate) async fn upsert_updown_session(
    pool: &SqlitePool,
    discord_user_id: serenity::UserId,
    thread_channel_id: serenity::ChannelId,
    pick_message_id: serenity::MessageId,
    current_level_tenths: i16,
    now_unix: i64,
) -> eyre::Result<()> {
    sqlx::query(
        r#"
INSERT INTO updown_sessions (
  discord_user_id,
  thread_channel_id,
  pick_message_id,
  current_level_tenths,
  created_at,
  updated_at
)
VALUES (?1, ?2, ?3, ?4, ?5, ?5)
ON CONFLICT(discord_user_id) DO UPDATE SET
  thread_channel_id = excluded.thread_channel_id,
  pick_message_id = excluded.pick_message_id,
  current_level_tenths = excluded.current_level_tenths,
  updated_at = excluded.updated_at
"#,
    )
    .bind(discord_user_id.to_string())
    .bind(thread_channel_id.to_string())
    .bind(pick_message_id.to_string())
    .bind(i64::from(current_level_tenths))
    .bind(now_unix)
    .execute(pool)
    .await
    .wrap_err("upsert updown session")?;
    Ok(())
}

pub(crate) async fn delete_updown_session(
    pool: &SqlitePool,
    discord_user_id: serenity::UserId,
) -> eyre::Result<()> {
    sqlx::query("DELETE FROM updown_sessions WHERE discord_user_id = ?1")
        .bind(discord_user_id.to_string())
        .execute(pool)
        .await
        .wrap_err("delete updown session")?;
    Ok(())
}

pub(crate) async fn list_updown_sessions(
    pool: &SqlitePool,
) -> eyre::Result<Vec<PersistedUpdownSession>> {
    let rows = sqlx::query_as::<_, (String, String, String, i64)>(
        r#"
SELECT discord_user_id, thread_channel_id, pick_message_id, current_level_tenths
FROM updown_sessions
"#,
    )
    .fetch_all(pool)
    .await
    .wrap_err("list updown sessions")?;

    let mut sessions = Vec::with_capacity(rows.len());
    for (user_id, thread_id, message_id, level_tenths) in rows {
        let parsed_user = user_id
            .parse::<u64>()
            .wrap_err("parse discord_user_id from updown_sessions")?;
        let parsed_thread = thread_id
            .parse::<u64>()
            .wrap_err("parse thread_channel_id from updown_sessions")?;
        let parsed_message = message_id
            .parse::<u64>()
            .wrap_err("parse pick_message_id from updown_sessions")?;
        let parsed_level: i16 = level_tenths
            .try_into()
            .wrap_err("parse current_level_tenths from updown_sessions")?;

        sessions.push(PersistedUpdownSession {
            discord_user_id: serenity::UserId::new(parsed_user),
            thread_channel_id: serenity::ChannelId::new(parsed_thread),
            pick_message_id: serenity::MessageId::new(parsed_message),
            current_level_tenths: parsed_level,
        });
    }

    Ok(sessions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn migrations_create_registration_table() -> eyre::Result<()> {
        let pool = connect("sqlite::memory:").await?;
        migrate(&pool).await?;

        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'discord_user_record_collectors'",
        )
        .fetch_one(&pool)
        .await?;

        assert_eq!(count, 1);
        Ok(())
    }

    #[tokio::test]
    async fn registration_crud_and_counts_work() -> eyre::Result<()> {
        let pool = connect("sqlite::memory:").await?;
        migrate(&pool).await?;

        let user_id = serenity::UserId::new(42);
        let other_user_id = serenity::UserId::new(99);

        assert_eq!(count_registrations(&pool).await?, 0);
        assert!(get_registration(&pool, user_id).await?.is_none());

        upsert_registration(&pool, user_id, "http://localhost:3000", 100).await?;
        assert_eq!(count_registrations(&pool).await?, 1);

        let registration = get_registration(&pool, user_id)
            .await?
            .expect("registration should exist");
        assert_eq!(registration.discord_user_id, user_id);
        assert_eq!(
            registration.record_collector_server_url,
            "http://localhost:3000"
        );

        upsert_registration(&pool, user_id, "https://collector.example", 200).await?;
        assert_eq!(count_registrations(&pool).await?, 1);

        let registration = get_registration(&pool, user_id)
            .await?
            .expect("registration should still exist");
        assert_eq!(
            registration.record_collector_server_url,
            "https://collector.example"
        );

        upsert_registration(&pool, other_user_id, "https://second.example", 300).await?;
        assert_eq!(count_registrations(&pool).await?, 2);

        Ok(())
    }
}
