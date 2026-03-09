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

#[cfg(test)]
mod tests {
    use super::*;
    use models::{StoredPlayRecord, StoredScoreEntry};

    #[test]
    fn score_response_preserves_plus_variants_from_storage() {
        let response = score_response_from_entry(StoredScoreEntry {
            title: "Test Song".to_string(),
            genre: "Genre".to_string(),
            artist: "Artist".to_string(),
            chart_type: "DX".to_string(),
            diff_category: "MASTER".to_string(),
            achievement_x10000: Some(1_005_000),
            rank: Some("SSS+".to_string()),
            fc: Some("FC+".to_string()),
            sync: Some("FS+".to_string()),
            dx_score: Some(1234),
            dx_score_max: Some(1500),
            last_played_at: None,
            play_count: Some(3),
        })
        .expect("score response should parse");

        let value = serde_json::to_value(response).expect("serialize score response");
        assert_eq!(value["rank"], "SSS+");
        assert_eq!(value["fc"], "FC+");
        assert_eq!(value["sync"], "FS+");
    }

    #[test]
    fn play_record_response_preserves_plus_variants_from_storage() {
        let response = play_record_response_from_record(StoredPlayRecord {
            played_at_unixtime: 1_700_000_000,
            played_at: Some("2026/03/09 21:00".to_string()),
            track: Some(1),
            title: "Test Song".to_string(),
            genre: Some("Genre".to_string()),
            artist: Some("Artist".to_string()),
            chart_type: "STD".to_string(),
            diff_category: Some("EXPERT".to_string()),
            achievement_x10000: Some(1_002_500),
            score_rank: Some("S+".to_string()),
            fc: Some("AP+".to_string()),
            sync: Some("FDX+".to_string()),
            dx_score: Some(1111),
            dx_score_max: Some(1500),
            credit_id: Some(7),
            achievement_new_record: Some(1),
        })
        .expect("play record response should parse");

        let value = serde_json::to_value(response).expect("serialize play record response");
        assert_eq!(value["score_rank"], "S+");
        assert_eq!(value["fc"], "AP+");
        assert_eq!(value["sync"], "FDX+");
    }
}
