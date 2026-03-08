use std::str::FromStr;

use crate::error::{AppError, Result};
use models::{
    ChartType, DifficultyCategory, FcStatus, ScoreRank, StoredPlayRecord, StoredScoreEntry,
    SyncStatus,
};
pub(crate) use models::{PlayRecordApiResponse, ScoreApiResponse};

pub(crate) fn score_response_from_entry(entry: StoredScoreEntry) -> Result<ScoreApiResponse> {
    let chart_type = entry.chart_type.parse::<ChartType>().ok().ok_or_else(|| {
        AppError::InternalError(format!("invalid chart_type '{}'", entry.chart_type))
    })?;
    let diff_category = entry
        .diff_category
        .parse::<DifficultyCategory>()
        .ok()
        .ok_or_else(|| {
            AppError::InternalError(format!("invalid diff_category '{}'", entry.diff_category))
        })?;

    let rank = parse_optional::<ScoreRank>(&entry.rank);
    let fc = parse_optional::<FcStatus>(&entry.fc);
    let sync = parse_optional::<SyncStatus>(&entry.sync);

    Ok(ScoreApiResponse {
        title: entry.title,
        genre: entry.genre,
        artist: entry.artist,
        chart_type,
        diff_category,
        achievement_x10000: entry.achievement_x10000,
        rank,
        fc,
        sync,
        dx_score: entry.dx_score,
        dx_score_max: entry.dx_score_max,
        last_played_at: entry.last_played_at,
        play_count: entry.play_count.and_then(|value| u32::try_from(value).ok()),
    })
}

pub(crate) fn play_record_response_from_record(
    record: StoredPlayRecord,
) -> Result<PlayRecordApiResponse> {
    let chart_type = record.chart_type.parse::<ChartType>().ok().ok_or_else(|| {
        AppError::InternalError(format!("invalid chart_type '{}'", record.chart_type))
    })?;
    let diff_category = record
        .diff_category
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(|s| {
            s.parse::<DifficultyCategory>()
                .ok()
                .ok_or_else(|| AppError::InternalError(format!("invalid diff_category '{}'", s)))
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
        genre: record.genre,
        artist: record.artist,
        chart_type,
        diff_category,
        achievement_x10000: record.achievement_x10000,
        score_rank,
        fc,
        sync,
        dx_score: record.dx_score,
        dx_score_max: record.dx_score_max,
        credit_id: record.credit_id,
        achievement_new_record: record.achievement_new_record,
    })
}

fn parse_optional<T: FromStr>(value: &Option<String>) -> Option<T> {
    value
        .as_deref()
        .filter(|s| !s.is_empty())
        .and_then(|s| s.parse().ok())
}
