use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedScoreEntry {
    pub song_key: String,
    pub title: String,
    pub diff: u8,
    pub achievement_percent: Option<f32>,
    pub rank: Option<String>,
    pub fc: Option<String>,
    pub sync: Option<String>,
    pub dx_score: Option<i32>,
    pub source_idx: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedPlayRecord {
    pub playlog_idx: Option<String>,
    pub track: Option<u8>,
    pub played_at: Option<String>,

    pub song_key: String,
    pub title: String,
    pub diff: Option<u8>,

    pub achievement_percent: Option<f32>,
    pub score_rank: Option<String>,
    pub fc: Option<String>,
    pub sync: Option<String>,
    pub dx_score: Option<i32>,
}
