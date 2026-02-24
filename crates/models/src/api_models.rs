use serde::{Deserialize, Serialize};

use crate::{ChartType, DifficultyCategory, FcStatus, ScoreRank, SyncStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreApiResponse {
    pub title: String,
    pub chart_type: ChartType,
    pub diff_category: DifficultyCategory,
    pub achievement_x10000: Option<i64>,
    pub rank: Option<ScoreRank>,
    pub fc: Option<FcStatus>,
    pub sync: Option<SyncStatus>,
    pub dx_score: Option<i32>,
    pub dx_score_max: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongDetailScoreApiResponse {
    pub title: String,
    pub chart_type: ChartType,
    pub diff_category: DifficultyCategory,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub achievement_x10000: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rank: Option<ScoreRank>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fc: Option<FcStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sync: Option<SyncStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dx_score: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dx_score_max: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_played_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub play_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayRecordApiResponse {
    pub played_at_unixtime: i64,
    pub played_at: Option<String>,
    pub track: Option<i32>,
    pub title: String,
    pub chart_type: ChartType,
    pub diff_category: Option<DifficultyCategory>,
    pub achievement_x10000: Option<i64>,
    pub score_rank: Option<ScoreRank>,
    pub fc: Option<FcStatus>,
    pub sync: Option<SyncStatus>,
    pub dx_score: Option<i32>,
    pub dx_score_max: Option<i32>,
    pub credit_play_count: Option<i32>,
    pub achievement_new_record: Option<i32>,
    pub first_play: Option<i32>,
}
