use std::path::PathBuf;

use eyre::{Result, WrapErr, bail};
use sha2::{Digest, Sha256};
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{Connection, SqliteConnection, SqlitePool};
use tempfile::tempdir;

#[derive(Debug)]
pub(crate) struct SnapshotArtifact {
    pub(crate) bytes: Vec<u8>,
    pub(crate) sha256_hex: String,
}

pub(crate) async fn create_snapshot(pool: &SqlitePool) -> Result<SnapshotArtifact> {
    let temp_dir = tempdir().wrap_err("create temp dir for sqlite backup")?;
    let snapshot_path = temp_dir.path().join("maimai-backup.sqlite3");

    let vacuum_sql = format!(
        "VACUUM main INTO '{}'",
        escape_sqlite_string(&snapshot_path.to_string_lossy())
    );
    sqlx::query(&vacuum_sql)
        .execute(pool)
        .await
        .wrap_err("create sqlite backup snapshot with VACUUM INTO")?;

    verify_integrity(&snapshot_path).await?;

    let bytes = tokio::fs::read(&snapshot_path)
        .await
        .wrap_err("read sqlite backup snapshot")?;
    let sha256_hex = hex::encode(Sha256::digest(&bytes));

    Ok(SnapshotArtifact { bytes, sha256_hex })
}

async fn verify_integrity(snapshot_path: &PathBuf) -> Result<()> {
    let options = SqliteConnectOptions::new()
        .filename(snapshot_path)
        .read_only(true);
    let mut conn = SqliteConnection::connect_with(&options)
        .await
        .wrap_err("connect to sqlite backup snapshot")?;
    let integrity: String = sqlx::query_scalar("PRAGMA integrity_check")
        .fetch_one(&mut conn)
        .await
        .wrap_err("run integrity_check on sqlite backup snapshot")?;
    conn.close()
        .await
        .wrap_err("close sqlite backup snapshot")?;

    if !integrity.eq_ignore_ascii_case("ok") {
        bail!("sqlite backup snapshot integrity_check failed: {integrity}");
    }

    Ok(())
}

fn escape_sqlite_string(value: &str) -> String {
    value.replace('\'', "''")
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn vacuum_into_creates_valid_snapshot() -> Result<()> {
        let temp_dir = tempdir()?;
        let db_path = temp_dir.path().join("test.sqlite3");

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(&db_path)
                    .create_if_missing(true),
            )
            .await?;
        sqlx::query("CREATE TABLE test_data(id INTEGER PRIMARY KEY, value TEXT NOT NULL)")
            .execute(&pool)
            .await?;
        sqlx::query("INSERT INTO test_data(value) VALUES ('hello')")
            .execute(&pool)
            .await?;

        let snapshot = create_snapshot(&pool).await?;

        assert!(!snapshot.bytes.is_empty());
        assert_eq!(snapshot.sha256_hex.len(), 64);

        Ok(())
    }
}
