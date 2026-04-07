use eyre::{Result, WrapErr};
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

/// Path to the Python plot script, relative to the working directory.
const PLOT_SCRIPT: &str = "scripts/mai_plot.py";

/// Generate a PNG scatter plot by delegating to the Python script via `uv run`.
///
/// * `points` — `(achievement_percent, level_tenths)` for each matching score
///   (e.g. `(100.5234, 130)` for a 13.0 chart scored 100.5234%).
/// * `x_min`  — left edge of the X axis (caller already clamped to ≤ 100.5).
pub(crate) async fn generate_scatter_plot(points: &[(f64, i32)], x_min: f64) -> Result<Vec<u8>> {
    let json_points: Vec<serde_json::Value> = points
        .iter()
        .map(|&(achievement, level_tenths)| {
            serde_json::json!({
                "achievement":   achievement,
                "level_tenths":  level_tenths,
            })
        })
        .collect();

    let payload = serde_json::json!({
        "points": json_points,
        "x_min":  x_min,
    });

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
