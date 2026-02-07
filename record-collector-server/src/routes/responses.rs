use std::str::FromStr;

use crate::error::{AppError, Result};
use crate::song_info_client::{SongInfoClient, SongMetadata};
use models::{
    ChartType, DifficultyCategory, FcStatus, PlayRecord, ScoreEntry, ScoreRank, SyncStatus,
};
pub use models::{PlayRecordResponse, ScoreResponse};

pub async fn score_response_from_entry(
    entry: ScoreEntry,
    song_info_client: &SongInfoClient,
) -> Result<ScoreResponse> {
    let chart_type = ChartType::from_str(&entry.chart_type).map_err(|e| {
        AppError::InternalError(format!("invalid chart_type '{}': {}", entry.chart_type, e))
    })?;
    let diff_category = DifficultyCategory::from_str(&entry.diff_category).map_err(|e| {
        AppError::InternalError(format!(
            "invalid diff_category '{}': {}",
            entry.diff_category, e
        ))
    })?;

    let metadata = song_info_client
        .get_song_metadata(&entry.title, chart_type.as_str(), diff_category.as_str())
        .await?;

    let effective_internal = metadata
        .internal_level
        .or_else(|| crate::rating::fallback_internal_level(&entry.level));

    let rating_points = if let (Some(internal), Some(ach_x10000)) =
        (effective_internal, entry.achievement_x10000)
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

    let rank = parse_optional::<ScoreRank>(&entry.rank);
    let fc = parse_optional::<FcStatus>(&entry.fc);
    let sync = parse_optional::<SyncStatus>(&entry.sync);

    Ok(ScoreResponse {
        title: entry.title,
        chart_type,
        diff_category,
        level: entry.level,
        achievement_x10000: entry.achievement_x10000,
        rank,
        fc,
        sync,
        dx_score: entry.dx_score,
        dx_score_max: entry.dx_score_max,
        source_idx: entry.source_idx,
        internal_level: effective_internal,
        image_name: metadata.image_name,
        version: metadata.version,
        rating_points,
        bucket: metadata.bucket,
    })
}

pub async fn play_record_response_from_record(
    record: PlayRecord,
    song_info_client: &SongInfoClient,
) -> Result<PlayRecordResponse> {
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

    let diff_category_str = diff_category.map(|d| d.as_str()).unwrap_or("");
    let metadata = if diff_category_str.is_empty() {
        SongMetadata::empty()
    } else {
        song_info_client
            .get_song_metadata(&record.title, chart_type.as_str(), diff_category_str)
            .await?
    };

    let effective_internal = metadata.internal_level.or_else(|| {
        record
            .level
            .as_deref()
            .and_then(crate::rating::fallback_internal_level)
    });

    let rating_points = if let (Some(internal), Some(ach_x10000)) =
        (effective_internal, record.achievement_x10000)
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

    let score_rank = parse_optional::<ScoreRank>(&record.score_rank);
    let fc = parse_optional::<FcStatus>(&record.fc);
    let sync = parse_optional::<SyncStatus>(&record.sync);

    Ok(PlayRecordResponse {
        played_at_unixtime: record.played_at_unixtime,
        played_at: record.played_at,
        track: record.track,
        title: record.title,
        chart_type,
        diff_category,
        level: record.level,
        achievement_x10000: record.achievement_x10000,
        score_rank,
        fc,
        sync,
        dx_score: record.dx_score,
        dx_score_max: record.dx_score_max,
        credit_play_count: record.credit_play_count,
        achievement_new_record: record.achievement_new_record,
        first_play: record.first_play,
        internal_level: effective_internal,
        rating_points,
        bucket: metadata.bucket,
    })
}

fn parse_optional<T: FromStr>(value: &Option<String>) -> Option<T> {
    value
        .as_deref()
        .filter(|s| !s.is_empty())
        .and_then(|s| s.parse().ok())
}
