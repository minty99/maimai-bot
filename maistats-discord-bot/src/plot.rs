use std::collections::HashMap;
use std::process::Stdio;

use eyre::{Result, WrapErr};
use maimai_client::SongCatalogSong;
use models::{ChartType, DifficultyCategory};
use time::{OffsetDateTime, UtcOffset};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

/// Path to the Python plot script, relative to the working directory.
const PLOT_SCRIPT: &str = "scripts/mai_plot.py";

/// Generate a PNG scatter plot by delegating to the Python script via `uv run`.
///
/// * `points` — `(achievement_percent, level_tenths, days_elapsed)` for each
///   matching score (e.g. `(100.5234, 130, 12.0)` for a 13.0 chart scored
///   100.5234% that was last played ~12 days ago). `days_elapsed` is used by
///   the renderer to fade older plays into the background.
/// * `x_min`  — left edge of the X axis (caller already clamped to ≤ 100.5).
/// * `title`  — optional title override; `None` uses the Python script default.
pub(crate) async fn generate_scatter_plot(
    points: &[(f64, i32, f64)],
    x_min: f64,
    title: Option<&str>,
) -> Result<Vec<u8>> {
    let json_points: Vec<serde_json::Value> = points
        .iter()
        .map(|&(achievement, level_tenths, days_elapsed)| {
            serde_json::json!({
                "achievement":   achievement,
                "level_tenths":  level_tenths,
                "days_elapsed":  days_elapsed,
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
