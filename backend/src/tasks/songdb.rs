use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};
use chrono_tz::Asia::Seoul;
use eyre::{ContextCompat, WrapErr};
use tokio::sync::Mutex;

use crate::state::AppState;

pub fn start_songdb_tasks(app_state: AppState) {
    let songdb_config = match maimai_songdb::SongDbConfig::from_env() {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("songdb: env not configured; skipping song DB updater: {e}");
            return;
        }
    };

    let songdb_config = Arc::new(songdb_config);

    let data_dir = PathBuf::from(&app_state.config.data_dir);
    let data_dir_for_startup = data_dir.clone();
    let app_state_for_startup = app_state.clone();
    let lock = Arc::new(Mutex::new(()));
    let lock_for_startup = lock.clone();
    let songdb_config_for_startup = songdb_config.clone();

    tokio::spawn(async move {
        let _guard = lock_for_startup.lock().await;
        
        let data_json_path = data_dir_for_startup
            .join(maimai_songdb::SONG_DATA_SUBDIR)
            .join("data.json");
        
        if data_json_path.exists() {
            tracing::info!("songdb: data.json already exists, skipping startup update");
            return;
        }
        
        tracing::info!("songdb: data.json not found, running initial update");
        if let Err(e) = run_update(&data_dir_for_startup, songdb_config_for_startup.as_ref()).await
        {
            tracing::warn!("songdb: startup update failed (non-fatal): {e:#}");
        } else {
            tracing::info!("songdb: startup update complete");
            if let Err(e) = app_state_for_startup.reload_song_data() {
                tracing::warn!("songdb: failed to reload song data after update: {e:#}");
            } else {
                tracing::info!("songdb: song data reloaded successfully");
            }
        }
    });

    let data_dir_for_loop = data_dir.clone();
    let app_state_for_loop = app_state;
    let lock_for_loop = lock;
    let songdb_config_for_loop = songdb_config;

    tokio::spawn(async move {
        if let Err(e) = run_daily_0730_kst_loop(
            &data_dir_for_loop,
            &app_state_for_loop,
            songdb_config_for_loop.as_ref(),
            lock_for_loop,
        )
        .await
        {
            tracing::warn!("songdb: scheduler task exited unexpectedly: {e:#}");
        }
    });
}

async fn run_update(data_dir: &Path, config: &maimai_songdb::SongDbConfig) -> eyre::Result<()> {
    tracing::info!("songdb: starting update...");

    let output_dir = data_dir.join(maimai_songdb::SONG_DATA_SUBDIR);
    std::fs::create_dir_all(&output_dir).wrap_err("create song_data output dir")?;

    let database = maimai_songdb::SongDatabase::fetch(config, &output_dir)
        .await
        .wrap_err("failed to fetch song database")?;

    let data_root = database
        .into_data_root()
        .wrap_err("failed to convert to data root")?;

    let json_bytes = serde_json::to_vec_pretty(&data_root).wrap_err("serialize data.json")?;
    std::fs::write(output_dir.join("data.json"), json_bytes).wrap_err("write data.json")?;

    Ok(())
}

async fn run_daily_0730_kst_loop(
    data_dir: &Path,
    app_state: &AppState,
    config: &maimai_songdb::SongDbConfig,
    lock: Arc<Mutex<()>>,
) -> eyre::Result<()> {
    loop {
        let now = Utc::now();
        let next_run = next_run_at_0730_kst(now).wrap_err("compute next songdb run")?;
        let sleep_for = next_run
            .signed_duration_since(now)
            .to_std()
            .wrap_err("next songdb run time is in the past")?;

        tokio::time::sleep(sleep_for).await;

        let _guard = lock.lock().await;
        match run_update(data_dir, config).await {
            Ok(_) => {
                tracing::info!("songdb: scheduled update complete");
                if let Err(e) = app_state.reload_song_data() {
                    tracing::warn!("songdb: failed to reload song data after update: {e:#}");
                } else {
                    tracing::info!("songdb: song data reloaded successfully");
                }
            }
            Err(e) => tracing::warn!("songdb: scheduled update failed (non-fatal): {e:#}"),
        }
    }
}

fn next_run_at_0730_kst(now_utc: DateTime<Utc>) -> eyre::Result<DateTime<Utc>> {
    let now_kst = now_utc.with_timezone(&Seoul);
    let today_run = Seoul
        .with_ymd_and_hms(now_kst.year(), now_kst.month(), now_kst.day(), 7, 30, 0)
        .single()
        .wrap_err("failed to resolve KST run time")?;
    let next_run = if now_kst < today_run {
        today_run
    } else {
        today_run + Duration::days(1)
    };
    Ok(next_run.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheduler_next_run_at_0730_kst() {
        let now_kst = Seoul.with_ymd_and_hms(2024, 1, 1, 6, 30, 0).unwrap();
        let expected = Seoul.with_ymd_and_hms(2024, 1, 1, 7, 30, 0).unwrap();
        let next_run = next_run_at_0730_kst(now_kst.with_timezone(&Utc)).expect("next_run");
        assert_eq!(next_run, expected.with_timezone(&Utc));

        let now_kst = Seoul.with_ymd_and_hms(2024, 1, 1, 7, 30, 0).unwrap();
        let expected = Seoul.with_ymd_and_hms(2024, 1, 2, 7, 30, 0).unwrap();
        let next_run = next_run_at_0730_kst(now_kst.with_timezone(&Utc)).expect("next_run");
        assert_eq!(next_run, expected.with_timezone(&Utc));

        let now_kst = Seoul.with_ymd_and_hms(2024, 1, 1, 8, 0, 0).unwrap();
        let expected = Seoul.with_ymd_and_hms(2024, 1, 2, 7, 30, 0).unwrap();
        let next_run = next_run_at_0730_kst(now_kst.with_timezone(&Utc)).expect("next_run");
        assert_eq!(next_run, expected.with_timezone(&Utc));

        let now_kst = Seoul.with_ymd_and_hms(2024, 1, 1, 23, 59, 59).unwrap();
        let expected = Seoul.with_ymd_and_hms(2024, 1, 2, 7, 30, 0).unwrap();
        let next_run = next_run_at_0730_kst(now_kst.with_timezone(&Utc)).expect("next_run");
        assert_eq!(next_run, expected.with_timezone(&Utc));
    }

    #[test]
    fn scheduler_next_run_handles_dst_transitions() {
        let now = Seoul
            .with_ymd_and_hms(2024, 6, 15, 8, 0, 0)
            .unwrap()
            .with_timezone(&Utc);
        let next_run = next_run_at_0730_kst(now).expect("next_run_at_0730_kst");
        let expected = Seoul
            .with_ymd_and_hms(2024, 6, 16, 7, 30, 0)
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(next_run, expected);
    }
}
