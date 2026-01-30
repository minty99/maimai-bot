use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;
use time::{Date, Duration as TimeDuration, Month, OffsetDateTime, UtcOffset};

use models::PlayRecord;
use crate::{error::Result, state::AppState};

#[derive(Deserialize)]
pub struct TodayQuery {
    day: Option<String>,
}

/// GET /api/today?day=YYYY-MM-DD
/// Query playlogs for a given day (default: today JST, day boundary 04:00)
/// Returns Vec<PlayRecord> ordered by played_at_unixtime ASC
pub async fn get_today(
    State(state): State<AppState>,
    Query(params): Query<TodayQuery>,
) -> Result<Json<Vec<PlayRecord>>> {
    let offset = UtcOffset::from_hms(9, 0, 0).unwrap_or(UtcOffset::UTC);

    // Parse day or use today (JST)
    let day_date = if let Some(date_str) = params.day.as_deref() {
        let key = date_str.trim().replace('-', "/");
        let parts = key.split('/').collect::<Vec<_>>();
        if parts.len() != 3 {
            return Err(crate::error::AppError::BadRequest(
                "date must be YYYY-MM-DD".to_string(),
            ));
        }
        let year = parts[0]
            .parse::<i32>()
            .map_err(|_| crate::error::AppError::BadRequest("invalid year".to_string()))?;
        let month = parts[1]
            .parse::<u8>()
            .map_err(|_| crate::error::AppError::BadRequest("invalid month".to_string()))?;
        let day = parts[2]
            .parse::<u8>()
            .map_err(|_| crate::error::AppError::BadRequest("invalid day".to_string()))?;
        let month = Month::try_from(month)
            .map_err(|_| crate::error::AppError::BadRequest("invalid month value".to_string()))?;
        Date::from_calendar_date(year, month, day)
            .map_err(|_| crate::error::AppError::BadRequest("invalid date".to_string()))?
    } else {
        let now_jst = OffsetDateTime::now_utc().to_offset(offset);
        if now_jst.hour() < 4 {
            (now_jst - TimeDuration::days(1)).date()
        } else {
            now_jst.date()
        }
    };

    let end_date = day_date + TimeDuration::days(1);

    // Format as "YYYY/MM/DD HH:MM" for comparison
    let start = format!(
        "{:04}/{:02}/{:02} 04:00",
        day_date.year(),
        u8::from(day_date.month()),
        day_date.day()
    );
    let end = format!(
        "{:04}/{:02}/{:02} 04:00",
        end_date.year(),
        u8::from(end_date.month()),
        end_date.day()
    );

    let rows = sqlx::query_as::<_, PlayRecord>(
        "SELECT 
            played_at_unixtime,
            played_at,
            track,
            title,
            chart_type,
            diff_category,
            level,
            achievement_x10000,
            score_rank,
            fc,
            sync,
            dx_score,
            dx_score_max,
            credit_play_count,
            achievement_new_record,
            first_play
         FROM playlogs
         WHERE played_at >= ? AND played_at < ?
         ORDER BY played_at_unixtime ASC",
    )
    .bind(&start)
    .bind(&end)
    .fetch_all(&state.db_pool)
    .await?;

    Ok(Json(rows))
}
