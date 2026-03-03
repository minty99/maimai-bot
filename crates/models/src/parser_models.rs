use serde::{Deserialize, Serialize};

use crate::{ChartType, DifficultyCategory, FcStatus, ScoreRank, SyncStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedScoreEntry {
    pub title: String,
    pub chart_type: ChartType,
    pub diff_category: DifficultyCategory,
    pub level: String,
    pub achievement_percent: Option<f32>,
    pub rank: Option<ScoreRank>,
    pub fc: Option<FcStatus>,
    pub sync: Option<SyncStatus>,
    pub dx_score: Option<i32>,
    pub dx_score_max: Option<i32>,
    pub last_played_at: Option<String>,
    pub play_count: Option<u32>,
    pub source_idx: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedPlayRecord {
    pub played_at_unixtime: Option<i64>,
    pub track: Option<u8>,
    pub played_at: Option<String>,
    pub credit_play_count: Option<u32>,
    pub title: String,
    pub chart_type: ChartType,
    pub diff_category: Option<DifficultyCategory>,
    pub level: Option<String>,
    pub achievement_percent: Option<f32>,
    pub achievement_new_record: bool,
    pub score_rank: Option<ScoreRank>,
    pub fc: Option<FcStatus>,
    pub sync: Option<SyncStatus>,
    pub dx_score: Option<i32>,
    pub dx_score_max: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedSongDetail {
    pub title: String,
    pub genre: Option<String>,
    pub chart_type: ChartType,
    pub difficulties: Vec<ParsedSongChartDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedSongChartDetail {
    pub diff_category: DifficultyCategory,
    pub level: String,
    pub chart_type: ChartType,
    pub achievement_percent: Option<f32>,
    pub rank: Option<ScoreRank>,
    pub fc: Option<FcStatus>,
    pub sync: Option<SyncStatus>,
    pub dx_score: Option<i32>,
    pub dx_score_max: Option<i32>,
    pub last_played_at: Option<String>,
    pub play_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedPlayerProfile {
    pub user_name: String,
    pub rating: u32,
    pub current_version_play_count: u32,
    pub total_play_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedRatingTargetEntry {
    pub title: String,
    pub chart_type: ChartType,
    pub diff_category: DifficultyCategory,
    pub level: String,
    pub achievement_percent: Option<f32>,
    pub rank: Option<ScoreRank>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedRatingTargets {
    pub current_targets: Vec<ParsedRatingTargetEntry>,
    pub legacy_targets: Vec<ParsedRatingTargetEntry>,
}
