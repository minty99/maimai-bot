use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct StoredScoreEntry {
    pub title: String,
    pub genre: String,
    pub artist: String,
    pub chart_type: String,
    pub diff_category: String,
    pub achievement_x10000: Option<i64>,
    pub rank: Option<String>,
    pub fc: Option<String>,
    pub sync: Option<String>,
    pub dx_score: Option<i32>,
    pub dx_score_max: Option<i32>,
    pub last_played_at: Option<String>,
    pub play_count: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct StoredPlayRecord {
    pub played_at_unixtime: i64,
    pub played_at: Option<String>,
    pub track: Option<i32>,
    pub title: String,
    pub genre: Option<String>,
    pub artist: Option<String>,
    pub chart_type: String,
    pub diff_category: Option<String>,
    pub achievement_x10000: Option<i64>,
    pub score_rank: Option<String>,
    pub fc: Option<String>,
    pub sync: Option<String>,
    pub dx_score: Option<i32>,
    pub dx_score_max: Option<i32>,
    pub credit_id: Option<i32>,
    pub achievement_new_record: Option<i32>,
}
