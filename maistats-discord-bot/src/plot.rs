use std::collections::HashMap;
use std::ops::RangeInclusive;
use std::process::Stdio;

use eyre::{Result, WrapErr};
use maimai_client::SongCatalogSong;
use models::{ChartType, DifficultyCategory};
use time::{OffsetDateTime, UtcOffset};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

/// Path to the Python plot script, relative to the working directory.
const PLOT_SCRIPT: &str = "scripts/mai_plot.py";

/// Minimum achievement (as `percent * 10000`) plotted on scatter charts.
/// Records below 90.0000% are excluded.
const MIN_ACHIEVEMENT_X10000: i64 = 900_000;

/// Normalized view of a single record fed into the scatter-plot pipeline.
///
/// Each source type (`PlayRecordApiResponse`, `ScoreApiResponse`, …) is
/// adapted into this shape by the caller. Fields that may be missing on the
/// upstream struct are modeled as `Option` so the shared pipeline can apply a
/// uniform "drop if unknown" rule without knowing the origin.
pub(crate) struct PlotInputRecord<'a> {
    pub achievement_x10000: Option<i64>,
    pub title: &'a str,
    pub genre: Option<&'a str>,
    pub artist: Option<&'a str>,
    pub chart_type: ChartType,
    pub diff_category: Option<DifficultyCategory>,
    pub played_at: Option<&'a str>,
    pub is_new_record: bool,
    pub previous_achievement_x10000: Option<i64>,
}

pub(crate) struct PlotPoint {
    pub achievement_percent: f64,
    pub level_tenths: i32,
    pub days_elapsed: f64,
    pub is_new_record: bool,
    pub previous_achievement_percent: Option<f64>,
}

/// Filters applied inside `build_plot_points`.
///
/// * `day_range` — when `Some`, records missing a valid `played_at` are
///   dropped and records outside the range are dropped. When `None`,
///   missing/malformed timestamps fall back to `0.0` days elapsed (used by
///   callers that already restrict the input to a known recent window).
/// * `level_range_tenths` — when `Some`, records whose internal level (in
///   tenths) fall outside the inclusive range are dropped.
pub(crate) struct PlotFilter {
    pub day_range: Option<RangeInclusive<f64>>,
    pub level_range_tenths: Option<RangeInclusive<i32>>,
}

/// Convert normalized records into scatter-plot points, applying the 90%
/// achievement floor plus the caller-supplied filter. Records are dropped
/// silently when any required field (achievement, genre, artist,
/// `diff_category`, level-map entry) is missing.
pub(crate) fn build_plot_points<'a>(
    records: impl IntoIterator<Item = PlotInputRecord<'a>>,
    level_map: &HashMap<LevelMapKey, i32>,
    now_jst: OffsetDateTime,
    jst: UtcOffset,
    filter: &PlotFilter,
) -> Vec<PlotPoint> {
    records
        .into_iter()
        .filter_map(|rec| {
            let achievement = rec.achievement_x10000?;
            if achievement < MIN_ACHIEVEMENT_X10000 {
                return None;
            }
            let genre = rec.genre?;
            let artist = rec.artist?;
            let diff_category = rec.diff_category?;
            let key = (
                rec.title.to_string(),
                genre.to_string(),
                artist.to_string(),
                rec.chart_type,
                diff_category,
            );
            let &il_tenths = level_map.get(&key)?;
            if let Some(range) = filter.level_range_tenths.as_ref()
                && !range.contains(&il_tenths)
            {
                return None;
            }
            let elapsed_days = match (
                filter.day_range.as_ref(),
                elapsed_days_since(rec.played_at, now_jst, jst),
            ) {
                (Some(range), Some(d)) if range.contains(&d) => d,
                (Some(_), _) => return None,
                (None, Some(d)) => d.max(0.0),
                (None, None) => 0.0,
            };
            Some(PlotPoint {
                achievement_percent: achievement as f64 / 10000.0,
                level_tenths: il_tenths,
                days_elapsed: elapsed_days,
                is_new_record: rec.is_new_record,
                previous_achievement_percent: rec
                    .previous_achievement_x10000
                    .filter(|previous| *previous >= MIN_ACHIEVEMENT_X10000)
                    .filter(|previous| *previous < achievement)
                    .map(|previous| previous as f64 / 10000.0),
            })
        })
        .collect()
}

/// Compute the X-axis left edge for a scatter plot of `(achievement_pct, _, _)`
/// points: the minimum achievement, capped at 100.5 so the axis always extends
/// through the perfect-play zone.
pub(crate) fn compute_x_min(points: &[PlotPoint]) -> f64 {
    points
        .iter()
        .flat_map(|point| {
            [
                Some(point.achievement_percent),
                point.previous_achievement_percent,
            ]
        })
        .flatten()
        .fold(f64::INFINITY, f64::min)
        .min(100.5)
}

/// Days between a JST-formatted timestamp and `now_jst`. Returns `None` when
/// the input is missing or malformed.
fn elapsed_days_since(
    played_at: Option<&str>,
    now_jst: OffsetDateTime,
    jst: UtcOffset,
) -> Option<f64> {
    played_at
        .and_then(|s| parse_jst_played_at(s, jst))
        .map(|dt| (now_jst - dt).as_seconds_f64() / (60.0 * 60.0 * 24.0))
}

/// Generate a PNG scatter plot by delegating to the Python script via `uv run`.
///
/// * `points` — plotted score points. `days_elapsed` is used by the renderer to
///   fade older plays into the background; new-record metadata lets the
///   renderer draw stars and optional previous-best arrows.
/// * `x_min`  — left edge of the X axis (caller already clamped to ≤ 100.5).
/// * `title`  — optional title override; `None` uses the Python script default.
pub(crate) async fn generate_scatter_plot(
    points: &[PlotPoint],
    x_min: f64,
    title: Option<&str>,
) -> Result<Vec<u8>> {
    let json_points: Vec<serde_json::Value> = points
        .iter()
        .map(|point| {
            serde_json::json!({
                "achievement":   point.achievement_percent,
                "level_tenths":  point.level_tenths,
                "days_elapsed":  point.days_elapsed,
                "is_new_record": point.is_new_record,
                "previous_achievement": point.previous_achievement_percent,
            })
        })
        .collect();

    let mut payload = serde_json::json!({
        "points": json_points,
        "x_min":  x_min,
    });
    if let Some(title) = title {
        payload["title"] = serde_json::Value::String(title.to_string());
    }

    let mut child = Command::new("uv")
        .args(["run", "--script", PLOT_SCRIPT])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .wrap_err("spawn uv plot script (is `uv` installed and on PATH?)")?;

    child
        .stdin
        .as_mut()
        .expect("stdin is piped")
        .write_all(payload.to_string().as_bytes())
        .await
        .wrap_err("write payload to plot script stdin")?;

    let output = child
        .wait_with_output()
        .await
        .wrap_err("wait for plot script to finish")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(eyre::eyre!(
            "plot script exited with error:\n{}",
            stderr.trim()
        ));
    }

    Ok(output.stdout)
}

/// Key used for looking up a chart's internal level in the song catalog.
pub(crate) type LevelMapKey = (String, String, String, ChartType, DifficultyCategory);

/// Build `(title, genre, artist, chart_type, diff_category) -> internal_level_tenths`
/// from the song catalog. Sheets without an internal level are skipped.
pub(crate) fn build_level_map(catalog: &[SongCatalogSong]) -> HashMap<LevelMapKey, i32> {
    let mut level_map: HashMap<LevelMapKey, i32> = HashMap::new();
    for song in catalog {
        for sheet in &song.sheets {
            if let Some(il) = sheet.internal_level {
                let il_tenths = (il * 10.0).round() as i32;
                level_map.insert(
                    (
                        song.title.clone(),
                        song.genre.clone(),
                        song.artist.clone(),
                        sheet.chart_type,
                        sheet.diff_category,
                    ),
                    il_tenths,
                );
            }
        }
    }
    level_map
}

/// Parse a `YYYY/MM/DD HH:MM` JST timestamp into an `OffsetDateTime`.
/// Returns `None` on malformed input; callers decide how to handle that.
pub(crate) fn parse_jst_played_at(s: &str, jst: UtcOffset) -> Option<OffsetDateTime> {
    if s.len() != 16 {
        return None;
    }
    let year: i32 = s.get(0..4)?.parse().ok()?;
    let month_num: u8 = s.get(5..7)?.parse().ok()?;
    let day: u8 = s.get(8..10)?.parse().ok()?;
    let hour: u8 = s.get(11..13)?.parse().ok()?;
    let minute: u8 = s.get(14..16)?.parse().ok()?;
    let month = time::Month::try_from(month_num).ok()?;
    let date = time::Date::from_calendar_date(year, month, day).ok()?;
    let tm = time::Time::from_hms(hour, minute, 0).ok()?;
    Some(date.with_time(tm).assume_offset(jst))
}

#[cfg(test)]
mod tests {
    use super::{LevelMapKey, PlotFilter, PlotInputRecord, build_plot_points};
    use models::{ChartType, DifficultyCategory};
    use std::collections::HashMap;
    use time::UtcOffset;

    #[test]
    fn build_plot_points_omits_previous_record_below_plot_floor() {
        let level_map = level_map();
        let points = build_plot_points(
            [PlotInputRecord {
                achievement_x10000: Some(910_000),
                title: "Song A",
                genre: Some("POPS"),
                artist: Some("Artist"),
                chart_type: ChartType::Dx,
                diff_category: Some(DifficultyCategory::Master),
                played_at: None,
                is_new_record: true,
                previous_achievement_x10000: Some(899_999),
            }],
            &level_map,
            time::OffsetDateTime::UNIX_EPOCH,
            UtcOffset::UTC,
            &PlotFilter {
                day_range: None,
                level_range_tenths: None,
            },
        );

        assert_eq!(points.len(), 1);
        assert_eq!(points[0].previous_achievement_percent, None);
    }

    fn level_map() -> HashMap<LevelMapKey, i32> {
        HashMap::from([(
            (
                "Song A".to_string(),
                "POPS".to_string(),
                "Artist".to_string(),
                ChartType::Dx,
                DifficultyCategory::Master,
            ),
            130,
        )])
    }
}
