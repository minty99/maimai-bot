use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};
use chrono_tz::Asia::Seoul;
use eyre::{ContextCompat, WrapErr};
use serde::Serialize;
use sqlx::SqlitePool;
use tokio::sync::Mutex;

#[derive(Debug, Serialize)]
struct SongDataRoot {
    songs: Vec<SongDataSong>,
}

#[derive(Debug, Serialize)]
struct SongDataSong {
    title: String,
    version: Option<String>,
    #[serde(rename = "imageName")]
    image_name: Option<String>,
    sheets: Vec<SongDataSheet>,
}

#[derive(Debug, Serialize)]
struct SongDataSheet {
    #[serde(rename = "type")]
    sheet_type: String,
    difficulty: String,
    #[serde(rename = "internalLevelValue")]
    internal_level_value: f32,
}

pub fn start_songdb_tasks(_db_pool: SqlitePool, data_dir: PathBuf) {
    let songdb_config = match maimai_songdb::SongDbConfig::from_env() {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("songdb: env not configured; skipping song DB updater: {e}");
            return;
        }
    };

    let songdb_config = Arc::new(songdb_config);

    let data_dir_for_startup = data_dir.clone();
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
        }
    });

    let data_dir_for_loop = data_dir.clone();
    let lock_for_loop = lock;
    let songdb_config_for_loop = songdb_config;

    tokio::spawn(async move {
        if let Err(e) = run_daily_0730_kst_loop(
            &data_dir_for_loop,
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

    let data = maimai_songdb::SongDatabase::fetch(config, &output_dir)
        .await
        .wrap_err("failed to fetch song database")?;

    let json_output = build_json_output(&data)?;
    let json_bytes = serde_json::to_vec_pretty(&json_output).wrap_err("serialize data.json")?;
    std::fs::write(output_dir.join("data.json"), json_bytes).wrap_err("write data.json")?;

    Ok(())
}

fn build_json_output(data: &maimai_songdb::SongDatabase) -> eyre::Result<SongDataRoot> {
    use std::collections::BTreeMap;

    let mut song_map: BTreeMap<String, SongDataSong> = BTreeMap::new();

    for song in &data.songs {
        song_map.insert(
            song.song_id.clone(),
            SongDataSong {
                title: song.title.clone(),
                version: song.version.clone(),
                image_name: Some(song.image_name.clone()),
                sheets: Vec::new(),
            },
        );
    }

    for sheet in &data.sheets {
        let song = match song_map.get_mut(&sheet.song_id) {
            Some(song) => song,
            None => continue,
        };

        let key = (
            sheet.song_id.clone(),
            sheet.sheet_type.clone(),
            sheet.difficulty.clone(),
        );
        let internal_level = data.internal_levels.get(&key);

        let Some(internal_level_str) = internal_level.map(|il| &il.internal_level) else {
            continue;
        };

        let internal_level_value = internal_level_str
            .trim()
            .parse::<f32>()
            .wrap_err("parse internal_level as f32")?;

        song.sheets.push(SongDataSheet {
            sheet_type: sheet.sheet_type.clone(),
            difficulty: sheet.difficulty.clone(),
            internal_level_value,
        });
    }

    Ok(SongDataRoot {
        songs: song_map.into_values().collect(),
    })
}

async fn run_daily_0730_kst_loop(
    data_dir: &Path,
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
            Ok(_) => tracing::info!("songdb: scheduled update complete"),
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
