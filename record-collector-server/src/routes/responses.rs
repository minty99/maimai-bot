use std::str::FromStr;

use crate::error::{AppError, Result};
use models::{
    ChartType, DifficultyCategory, FcStatus, ScoreRank, StoredPlayRecord, StoredScoreEntry,
    SyncStatus,
};
pub(crate) use models::{PlayRecordApiResponse, ScoreApiResponse};

pub(crate) fn score_response_from_entry(entry: StoredScoreEntry) -> Result<ScoreApiResponse> {
    let chart_type = ChartType::from_str(&entry.chart_type).map_err(|e| {
        AppError::InternalError(format!("invalid chart_type '{}': {}", entry.chart_type, e))
    })?;
    let diff_category = DifficultyCategory::from_str(&entry.diff_category).map_err(|e| {
        AppError::InternalError(format!(
            "invalid diff_category '{}': {}",
            entry.diff_category, e
        ))
    })?;

    let rank = parse_optional::<ScoreRank>(&entry.rank);
    let fc = parse_optional::<FcStatus>(&entry.fc);
    let sync = parse_optional::<SyncStatus>(&entry.sync);

    Ok(ScoreApiResponse {
        title: entry.title,
        chart_type,
        diff_category,
        achievement_x10000: entry.achievement_x10000,
        rank,
        fc,
        sync,
        dx_score: entry.dx_score,
        dx_score_max: entry.dx_score_max,
    })
}

pub(crate) fn play_record_response_from_record(
    record: StoredPlayRecord,
) -> Result<PlayRecordApiResponse> {
    let chart_type = ChartType::from_str(&record.chart_type).map_err(|e| {
        AppError::InternalError(format!("invalid chart_type '{}': {}", record.chart_type, e))
    })?;
    let diff_category = record
        .diff_category
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(|s| {
            DifficultyCategory::from_str(s).map_err(|e| {
                AppError::InternalError(format!("invalid diff_category '{}': {}", s, e))
            })
        })
        .transpose()?;

    let score_rank = parse_optional::<ScoreRank>(&record.score_rank);
    let fc = parse_optional::<FcStatus>(&record.fc);
    let sync = parse_optional::<SyncStatus>(&record.sync);

    Ok(PlayRecordApiResponse {
        played_at_unixtime: record.played_at_unixtime,
        played_at: record.played_at,
        track: record.track,
        title: record.title,
        chart_type,
        diff_category,
        achievement_x10000: record.achievement_x10000,
        score_rank,
        fc,
        sync,
        dx_score: record.dx_score,
        dx_score_max: record.dx_score_max,
        credit_play_count: record.credit_play_count,
        achievement_new_record: record.achievement_new_record,
        first_play: record.first_play,
    })
}

fn parse_optional<T: FromStr>(value: &Option<String>) -> Option<T> {
    value
        .as_deref()
        .filter(|s| !s.is_empty())
        .and_then(|s| s.parse().ok())
}
