use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{Datelike, Utc};
use eyre::{Result, WrapErr};
use sqlx::SqlitePool;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::backup::s3::S3Uploader;
use crate::backup::snapshot::create_snapshot;
use crate::config::BackupConfig;
use crate::db::{get_app_state_string, get_app_state_u32, set_app_state_string};
use crate::tasks::sync_shared::STATE_KEY_TOTAL_PLAY_COUNT;

const STATE_KEY_LAST_UPLOADED_AT: &str = "backup.last_uploaded_at";
const STATE_KEY_LAST_UPLOADED_KEY: &str = "backup.last_uploaded_key";
const STATE_KEY_LAST_UPLOADED_PLAY_COUNT: &str = "backup.last_uploaded_play_count";
const STATE_KEY_LAST_UPLOADED_SHA256: &str = "backup.last_uploaded_sha256";
const STATE_KEY_LAST_ERROR: &str = "backup.last_error";
const STATE_KEY_LAST_ERROR_AT: &str = "backup.last_error_at";

#[derive(Debug, Clone, Copy)]
pub(crate) enum BackupReason {
    StartupSync,
    PeriodicSync,
}

#[derive(Debug, Clone, Copy)]
struct BackupRequest {
    reason: BackupReason,
    play_count_hint: Option<u32>,
}

pub(crate) struct BackupService {
    sender: mpsc::UnboundedSender<BackupRequest>,
}

impl BackupService {
    pub(crate) async fn new(config: BackupConfig, db_pool: SqlitePool) -> Result<Arc<Self>> {
        let uploader = S3Uploader::from_config(&config)
            .await
            .wrap_err("create S3 uploader")?;
        let (sender, receiver) = mpsc::unbounded_channel();
        let service = Arc::new(Self { sender });

        tokio::spawn(run_backup_worker(db_pool, uploader, receiver));

        Ok(service)
    }

    pub(crate) fn request_backup(&self, reason: BackupReason, play_count_hint: Option<u32>) {
        if let Err(err) = self.sender.send(BackupRequest {
            reason,
            play_count_hint,
        }) {
            warn!("Backup request dropped because worker is unavailable: {err}");
        }
    }
}

async fn run_backup_worker(
    db_pool: SqlitePool,
    uploader: S3Uploader,
    mut receiver: mpsc::UnboundedReceiver<BackupRequest>,
) {
    while let Some(request) = receiver.recv().await {
        let mut merged_request = request;
        while let Ok(next_request) = receiver.try_recv() {
            if next_request.play_count_hint.is_some() {
                merged_request.play_count_hint = next_request.play_count_hint;
            }
            merged_request.reason = next_request.reason;
        }

        if let Err(err) = perform_backup_once(&db_pool, &uploader, merged_request).await {
            error!("SQLite backup upload failed: {err:#}");
            if let Err(store_err) = store_backup_error(&db_pool, &err.to_string()).await {
                error!("Failed to persist backup error state: {store_err:#}");
            }
        }
    }
}

async fn perform_backup_once(
    db_pool: &SqlitePool,
    uploader: &S3Uploader,
    request: BackupRequest,
) -> Result<()> {
    let snapshot = create_snapshot(db_pool)
        .await
        .wrap_err("create sqlite backup snapshot")?;

    let last_sha256 = get_app_state_string(db_pool, STATE_KEY_LAST_UPLOADED_SHA256)
        .await
        .wrap_err("load last backup sha256")?;
    if should_skip_upload(last_sha256.as_deref(), &snapshot.sha256_hex) {
        debug!(
            "Skipping SQLite backup upload because snapshot SHA matches previous upload: reason={:?}",
            request.reason
        );
        return Ok(());
    }

    let play_count = resolve_play_count(db_pool, request.play_count_hint)
        .await
        .wrap_err("resolve play count for backup key")?;
    let uploaded_at = unix_timestamp();
    let object_key = build_object_key(uploaded_at, play_count, &snapshot.sha256_hex);
    let full_key = uploader.prefixed_key(&object_key);

    uploader
        .upload_snapshot(&full_key, snapshot.bytes)
        .await
        .wrap_err("upload snapshot object")?;

    info!(
        "SQLite backup uploaded to S3: reason={:?} key={} play_count={}",
        request.reason, full_key, play_count
    );

    persist_backup_success(
        db_pool,
        &full_key,
        play_count,
        &snapshot.sha256_hex,
        uploaded_at,
    )
    .await
    .wrap_err("persist backup success state")?;

    Ok(())
}

async fn resolve_play_count(db_pool: &SqlitePool, play_count_hint: Option<u32>) -> Result<u32> {
    if let Some(play_count) = play_count_hint {
        return Ok(play_count);
    }

    Ok(get_app_state_u32(db_pool, STATE_KEY_TOTAL_PLAY_COUNT)
        .await
        .wrap_err("load stored total play count for backup")?
        .unwrap_or(0))
}

async fn persist_backup_success(
    db_pool: &SqlitePool,
    object_key: &str,
    play_count: u32,
    sha256_hex: &str,
    uploaded_at: i64,
) -> Result<()> {
    set_app_state_string(
        db_pool,
        STATE_KEY_LAST_UPLOADED_SHA256,
        sha256_hex,
        uploaded_at,
    )
    .await?;
    set_app_state_string(
        db_pool,
        STATE_KEY_LAST_UPLOADED_PLAY_COUNT,
        &play_count.to_string(),
        uploaded_at,
    )
    .await?;
    set_app_state_string(
        db_pool,
        STATE_KEY_LAST_UPLOADED_AT,
        &uploaded_at.to_string(),
        uploaded_at,
    )
    .await?;
    set_app_state_string(
        db_pool,
        STATE_KEY_LAST_UPLOADED_KEY,
        object_key,
        uploaded_at,
    )
    .await?;
    set_app_state_string(db_pool, STATE_KEY_LAST_ERROR, "", uploaded_at).await?;
    set_app_state_string(db_pool, STATE_KEY_LAST_ERROR_AT, "", uploaded_at).await?;
    Ok(())
}

async fn store_backup_error(db_pool: &SqlitePool, error_message: &str) -> Result<()> {
    let updated_at = unix_timestamp();
    set_app_state_string(db_pool, STATE_KEY_LAST_ERROR, error_message, updated_at).await?;
    set_app_state_string(
        db_pool,
        STATE_KEY_LAST_ERROR_AT,
        &updated_at.to_string(),
        updated_at,
    )
    .await?;
    Ok(())
}

fn should_skip_upload(last_sha256: Option<&str>, current_sha256: &str) -> bool {
    matches!(last_sha256, Some(last) if last == current_sha256)
}

fn build_object_key(uploaded_at: i64, play_count: u32, sha256_hex: &str) -> String {
    let now = Utc::now();
    let sha_prefix = sha256_hex.get(..12).unwrap_or(sha256_hex);
    format!(
        "{:04}/{:02}/{:02}/maimai-{}-playcount-{}-{}.sqlite3",
        now.year(),
        now.month(),
        now.day(),
        uploaded_at,
        play_count,
        sha_prefix
    )
}

fn unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedupe_uses_sha256_match() {
        assert!(should_skip_upload(Some("abc"), "abc"));
        assert!(!should_skip_upload(Some("abc"), "def"));
        assert!(!should_skip_upload(None, "def"));
    }

    #[test]
    fn object_key_contains_date_play_count_and_hash_prefix() {
        let key = build_object_key(1_772_504_101, 781, "a1b2c3d4e5f6a7b8c9d0");
        assert!(key.ends_with("maimai-1772504101-playcount-781-a1b2c3d4e5f6.sqlite3"));
        assert!(key.starts_with("20"));
    }
}
