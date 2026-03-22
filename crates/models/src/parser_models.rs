use serde::{Deserialize, Serialize};

use crate::{ChartType, DifficultyCategory, FcStatus, ScoreRank, SyncStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedScoreEntry {
    pub title: String,
    pub genre: String,
    pub artist: String,
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

impl ParsedScoreEntry {
    pub fn format_recent_sync_log_fields(&self) -> String {
        format!(
            "title='{}' genre='{}' artist='{}' chart_type={} diff_category={} last_played_at='{}' play_count={} achievement_x10000={} rank={} fc={} sync={} dx_score={}/{}",
            self.title,
            self.genre,
            self.artist,
            self.chart_type.as_str(),
            self.diff_category.as_str(),
            display_opt_str(self.last_played_at.as_deref()),
            display_opt_u32(self.play_count),
            display_opt_i64(percent_to_x10000(self.achievement_percent)),
            display_opt_score_rank(self.rank),
            display_opt_fc(self.fc),
            display_opt_sync(self.sync),
            display_opt_i32(self.dx_score),
            display_opt_i32(self.dx_score_max),
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedPlayRecord {
    pub played_at_unixtime: Option<i64>,
    pub playlog_detail_idx: Option<String>,
    pub track: Option<u8>,
    pub played_at: Option<String>,
    pub credit_id: Option<u32>,
    pub title: String,
    pub genre: Option<String>,
    pub artist: Option<String>,
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

impl ParsedPlayRecord {
    pub fn format_recent_sync_log_fields(&self) -> String {
        format!(
            "played_at_unixtime={} played_at='{}' credit_id={} track={} title='{}' genre='{}' artist='{}' chart_type={} diff_category={} achievement_x10000={} new_record={} rank={} fc={} sync={} dx_score={}/{}",
            display_opt_i64(self.played_at_unixtime),
            display_opt_str(self.played_at.as_deref()),
            display_opt_u32(self.credit_id),
            display_opt_u8(self.track),
            self.title,
            display_opt_str(self.genre.as_deref()),
            display_opt_str(self.artist.as_deref()),
            self.chart_type.as_str(),
            display_opt_diff_category(self.diff_category),
            display_opt_i64(percent_to_x10000(self.achievement_percent)),
            self.achievement_new_record,
            display_opt_score_rank(self.score_rank),
            display_opt_fc(self.fc),
            display_opt_sync(self.sync),
            display_opt_i32(self.dx_score),
            display_opt_i32(self.dx_score_max),
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedSongDetail {
    pub title: String,
    pub genre: Option<String>,
    pub artist: String,
    pub chart_type: ChartType,
    pub difficulties: Vec<ParsedSongChartDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedPlaylogDetail {
    pub title: String,
    pub music_detail_idx: String,
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

fn percent_to_x10000(percent: Option<f32>) -> Option<i64> {
    percent.map(|p| (p as f64 * 10000.0).round() as i64)
}

fn display_opt_str(value: Option<&str>) -> &str {
    value.unwrap_or("-")
}

fn display_opt_i64(value: Option<i64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn display_opt_u32(value: Option<u32>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn display_opt_u8(value: Option<u8>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn display_opt_i32(value: Option<i32>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn display_opt_diff_category(value: Option<DifficultyCategory>) -> &'static str {
    value.map(|value| value.as_str()).unwrap_or("-")
}

fn display_opt_score_rank(value: Option<ScoreRank>) -> &'static str {
    value.map(|value| value.as_str()).unwrap_or("-")
}

fn display_opt_fc(value: Option<FcStatus>) -> &'static str {
    value.map(|value| value.as_str()).unwrap_or("-")
}

fn display_opt_sync(value: Option<SyncStatus>) -> &'static str {
    value.map(|value| value.as_str()).unwrap_or("-")
}
