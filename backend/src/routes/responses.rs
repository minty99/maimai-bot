use crate::state::AppState;
use models::{PlayRecord, ScoreEntry};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreResponse {
    pub title: String,
    pub chart_type: String,
    pub diff_category: String,
    pub level: String,
    pub achievement_x10000: Option<i64>,
    pub rank: Option<String>,
    pub fc: Option<String>,
    pub sync: Option<String>,
    pub dx_score: Option<i32>,
    pub dx_score_max: Option<i32>,
    pub source_idx: Option<String>,
    pub internal_level: Option<f32>,
    pub image_name: Option<String>,
    pub rating_points: Option<u32>,
    pub bucket: Option<String>,
}

impl ScoreResponse {
    pub fn from_entry(entry: ScoreEntry, state: &AppState) -> Self {
        let song_data = state.song_data.read().unwrap();

        let internal_level =
            song_data.internal_level(&entry.title, &entry.chart_type, &entry.diff_category);
        let image_name = song_data.image_name(&entry.title).map(|s| s.to_string());

        let rating_points = if let (Some(internal), Some(ach_x10000)) =
            (internal_level, entry.achievement_x10000)
        {
            let achievement_percent = ach_x10000 as f64 / 10000.0;
            let ap_bonus = crate::rating::is_ap_like(entry.fc.as_deref());
            Some(crate::rating::chart_rating_points(
                internal as f64,
                achievement_percent,
                ap_bonus,
            ))
        } else {
            None
        };

        let bucket = song_data.bucket(&entry.title).map(|b| match b {
            models::SongBucket::New => "New".to_string(),
            models::SongBucket::Old => "Old".to_string(),
        });

        Self {
            title: entry.title,
            chart_type: entry.chart_type,
            diff_category: entry.diff_category,
            level: entry.level,
            achievement_x10000: entry.achievement_x10000,
            rank: entry.rank,
            fc: entry.fc,
            sync: entry.sync,
            dx_score: entry.dx_score,
            dx_score_max: entry.dx_score_max,
            source_idx: entry.source_idx,
            internal_level,
            image_name,
            rating_points,
            bucket,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayRecordResponse {
    pub played_at_unixtime: i64,
    pub played_at: Option<String>,
    pub track: Option<i32>,
    pub title: String,
    pub chart_type: String,
    pub diff_category: Option<String>,
    pub level: Option<String>,
    pub achievement_x10000: Option<i64>,
    pub score_rank: Option<String>,
    pub fc: Option<String>,
    pub sync: Option<String>,
    pub dx_score: Option<i32>,
    pub dx_score_max: Option<i32>,
    pub credit_play_count: Option<i32>,
    pub achievement_new_record: Option<i32>,
    pub first_play: Option<i32>,
    pub internal_level: Option<f32>,
    pub rating_points: Option<u32>,
    pub bucket: Option<String>,
}

impl PlayRecordResponse {
    pub fn from_record(record: PlayRecord, state: &AppState) -> Self {
        let song_data = state.song_data.read().unwrap();

        let internal_level = song_data.internal_level(
            &record.title,
            &record.chart_type,
            record.diff_category.as_deref().unwrap_or(""),
        );

        let rating_points = if let (Some(internal), Some(ach_x10000)) =
            (internal_level, record.achievement_x10000)
        {
            let achievement_percent = ach_x10000 as f64 / 10000.0;
            let ap_bonus = crate::rating::is_ap_like(record.fc.as_deref());
            Some(crate::rating::chart_rating_points(
                internal as f64,
                achievement_percent,
                ap_bonus,
            ))
        } else {
            None
        };

        let bucket = song_data.bucket(&record.title).map(|b| match b {
            models::SongBucket::New => "New".to_string(),
            models::SongBucket::Old => "Old".to_string(),
        });

        Self {
            played_at_unixtime: record.played_at_unixtime,
            played_at: record.played_at,
            track: record.track,
            title: record.title,
            chart_type: record.chart_type,
            diff_category: record.diff_category,
            level: record.level,
            achievement_x10000: record.achievement_x10000,
            score_rank: record.score_rank,
            fc: record.fc,
            sync: record.sync,
            dx_score: record.dx_score,
            dx_score_max: record.dx_score_max,
            credit_play_count: record.credit_play_count,
            achievement_new_record: record.achievement_new_record,
            first_play: record.first_play,
            internal_level,
            rating_points,
            bucket,
        }
    }
}
