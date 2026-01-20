use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ChartType {
    Std,
    Dx,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedScoreEntry {
    pub song_key: String,
    pub title: String,
    pub chart_type: ChartType,
    pub diff_category: String,
    pub level: String,
    pub achievement_percent: Option<f32>,
    pub rank: Option<String>,
    pub fc: Option<String>,
    pub sync: Option<String>,
    pub dx_score: Option<i32>,
    pub dx_score_max: Option<i32>,
    pub source_idx: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedPlayRecord {
    pub playlog_idx: Option<String>,
    pub track: Option<u8>,
    pub played_at: Option<String>,

    pub song_key: String,
    pub title: String,
    pub chart_type: ChartType,
    pub diff_category: Option<String>,
    pub level: Option<String>,

    pub achievement_percent: Option<f32>,
    pub score_rank: Option<String>,
    pub fc: Option<String>,
    pub sync: Option<String>,
    pub dx_score: Option<i32>,
    pub dx_score_max: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedSongDetail {
    pub song_key: String,
    pub title: String,
    pub chart_type: ChartType,
    pub difficulties: Vec<ParsedSongDifficultyDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedSongDifficultyDetail {
    pub diff_category: String,
    pub level: String,
    pub chart_type: ChartType,
    pub achievement_percent: Option<f32>,
    pub rank: Option<String>,
    pub fc: Option<String>,
    pub sync: Option<String>,
    pub dx_score: Option<i32>,
    pub dx_score_max: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedPlayerData {
    pub user_name: String,
    pub rating: u32,
    pub current_version_play_count: u32,
    pub total_play_count: u32,
}
