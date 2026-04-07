use eyre::{Result, WrapErr};
use plotters::prelude::*;
use rand::{Rng, SeedableRng};
use std::collections::HashMap;

/// D3 qualitative palette — 10 distinct colours (hex: #1f77b4, …).
const D3: [(u8, u8, u8); 10] = [
    (0x1f, 0x77, 0xb4),
    (0xff, 0x7f, 0x0e),
    (0x2c, 0xa0, 0x2c),
    (0xd6, 0x27, 0x28),
    (0x94, 0x67, 0xbd),
    (0x8c, 0x56, 0x4b),
    (0xe3, 0x77, 0xc2),
    (0x7f, 0x7f, 0x7f),
    (0xbc, 0xbd, 0x22),
    (0x17, 0xbe, 0xcf),
];

const RANK_THRESHOLDS: &[(f64, &str)] = &[
    (97.0, "S"),
    (98.0, "S+"),
    (99.0, "SS"),
    (99.5, "SS+"),
    (100.0, "SSS"),
    (100.5, "SSS+"),
];

const JITTER: f64 = 0.35;
/// Render at 2× for HiDPI displays.
const SCALE: u32 = 2;
const BASE_HEIGHT: u32 = 650;

// Pre-blended colours (alpha-composited onto white background):
//   rank lines  rgba(130,130,130,0.75) → rgb(161,161,161)
//   lane seps   rgba(180,180,180,0.55) → rgb(213,213,213)
//   y-grid      rgba(200,200,200,0.5)  → rgb(227,227,227)
const RANK_LINE: RGBColor = RGBColor(161, 161, 161);
const LANE_SEP: RGBColor = RGBColor(213, 213, 213);
const GRID: RGBColor = RGBColor(227, 227, 227);

/// Generate a PNG scatter-plot.
///
/// * `points` – `(achievement_percent, level_tenths)` for each qualifying score.
/// * `y_min`  – lower bound of the achievement (Y) axis.
pub(crate) fn generate_scatter_plot(points: &[(f64, i32)], y_min: f64) -> Result<Vec<u8>> {
    // ── levels & layout ─────────────────────────────────────────────────────
    let mut levels: Vec<i32> = points.iter().map(|&(_, lt)| lt).collect();
    levels.sort_unstable();
    levels.dedup();
    let n = levels.len();

    let level_idx: HashMap<i32, usize> =
        levels.iter().enumerate().map(|(i, &lt)| (lt, i)).collect();

    let base_w = (110 * n as u32 + 220).clamp(450, 1200);
    let w = base_w * SCALE;
    let h = BASE_HEIGHT * SCALE;

    // Margins matching the previous plotly layout (logical px → scaled px):
    //   plotly: l=70, r=110, t=55, b=50
    let m_l = 70u32 * SCALE;
    let m_r = 110u32 * SCALE;
    let m_t = 55u32 * SCALE;
    let m_b = 50u32 * SCALE;

    // ── reproducible jitter ─────────────────────────────────────────────────
    let mut rng = rand::rngs::StdRng::seed_from_u64(0);
    let jittered: Vec<(f64, f64, i32)> = points
        .iter()
        .map(|&(ach, lt)| {
            let x = level_idx[&lt] as f64 + rng.gen_range(-JITTER..=JITTER);
            (x, ach, lt)
        })
        .collect();

    // ── chart coordinate ranges ─────────────────────────────────────────────
    let x_lo = -0.5_f64;
    let x_hi = n as f64 - 0.5;
    let y_lo = y_min;
    let y_hi = 101.0_f64;

    // ── draw ────────────────────────────────────────────────────────────────
    let mut buf = vec![0u8; (w * h * 3) as usize];
    {
        let root = BitMapBackend::with_buffer(&mut buf, (w, h)).into_drawing_area();
        root.fill(&WHITE).wrap_err("fill background")?;

        // Title
        let n_pts = points.len();
        let level_label = if n == 1 {
            format!("Lv {:.1}", levels[0] as f64 / 10.0)
        } else {
            format!(
                "Lv {:.1}\u{2013}{:.1}",
                levels[0] as f64 / 10.0,
                levels[n - 1] as f64 / 10.0
            )
        };
        let title = format!(
            "{}  \u{2014}  {} song{} (last 3 months, \u{2265}90%)",
            level_label,
            n_pts,
            if n_pts == 1 { "" } else { "s" }
        );
        root.draw_text(
            &title,
            &("sans-serif", (13 * SCALE) as i32)
                .into_font()
                .color(&BLACK),
            (m_l as i32, (m_t / 3) as i32),
        )
        .wrap_err("draw title")?;

        // Build chart
        let mut chart = ChartBuilder::on(&root)
            .margin_top(m_t)
            .margin_bottom(m_b)
            .margin_left(m_l)
            .margin_right(m_r)
            .build_cartesian_2d(x_lo..x_hi, y_lo..y_hi)
            .wrap_err("build chart")?;

        let label_style = ("sans-serif", (9 * SCALE) as i32)
            .into_font()
            .color(&RGBColor(80, 80, 80));

        chart
            .configure_mesh()
            .disable_x_mesh()
            .light_line_style(GRID)
            .y_labels(12)
            .y_label_formatter(&|v: &f64| format!("{:.2}", v))
            .y_label_style(label_style.clone())
            .y_desc("Achievement %")
            // x labels: plotters places n ticks over range width n → step 1.0
            // (maps neatly to lane indices 0…n-1)
            .x_labels(n)
            .x_label_formatter(&|v: &f64| {
                let idx = (v.round() as isize).clamp(0, n as isize - 1) as usize;
                format!("{:.1}", levels[idx] as f64 / 10.0)
            })
            .x_label_style(label_style.clone())
            .axis_desc_style(label_style.clone())
            .draw()
            .wrap_err("draw mesh")?;

        // ── rank boundary lines + right-side labels ──────────────────────
        for &(rank_val, label) in RANK_THRESHOLDS {
            if rank_val < y_lo || rank_val > y_hi + 0.001 {
                continue;
            }
            // Dotted line: alternate short drawn/gap segments in chart space
            let seg = (x_hi - x_lo) / 80.0;
            let mut x = x_lo;
            while x < x_hi {
                let end = (x + seg).min(x_hi);
                chart
                    .draw_series(LineSeries::new(
                        [(x, rank_val), (end, rank_val)],
                        RANK_LINE.stroke_width(SCALE),
                    ))
                    .wrap_err("draw rank segment")?;
                x += 2.0 * seg; // seg drawn, seg gap
            }
            // Label in right margin, vertically centred on the line
            let (px, py) = chart.backend_coord(&(x_hi, rank_val));
            root.draw_text(
                label,
                &("sans-serif", (9 * SCALE) as i32)
                    .into_font()
                    .color(&RGBColor(90, 90, 90)),
                (px + (6 * SCALE) as i32, py - (6 * SCALE) as i32),
            )
            .wrap_err("draw rank label")?;
        }

        // ── lane separator lines (dashed) ────────────────────────────────
        for i in 1..n {
            let lx = i as f64 - 0.5;
            let seg = (y_hi - y_lo) / 30.0;
            let mut y = y_lo;
            while y < y_hi {
                let end = (y + seg).min(y_hi);
                chart
                    .draw_series(LineSeries::new(
                        [(lx, y), (lx, end)],
                        LANE_SEP.stroke_width(SCALE),
                    ))
                    .wrap_err("draw lane separator")?;
                y += seg + seg * 0.6; // seg drawn, 0.6×seg gap
            }
        }

        // ── scatter points ───────────────────────────────────────────────
        let dot_r = (5 * SCALE) as i32;
        let border_r = dot_r + SCALE as i32; // 1-px white ring
        for &lt in &levels {
            let (r, g, b) = D3[level_idx[&lt] % D3.len()];
            let fill = RGBColor(r, g, b);
            // White border ring (drawn first, underneath)
            chart
                .draw_series(
                    jittered
                        .iter()
                        .filter(|&&(_, _, l)| l == lt)
                        .map(|&(x, y, _)| Circle::new((x, y), border_r, WHITE.filled())),
                )
                .wrap_err("draw border circles")?;
            // Filled dot
            chart
                .draw_series(
                    jittered
                        .iter()
                        .filter(|&&(_, _, l)| l == lt)
                        .map(|&(x, y, _)| Circle::new((x, y), dot_r, fill.filled())),
                )
                .wrap_err("draw scatter dots")?;
        }
    }

    // ── encode raw RGB pixels → PNG ──────────────────────────────────────
    let img = image::RgbImage::from_raw(w, h, buf)
        .ok_or_else(|| eyre::eyre!("pixel buffer size mismatch"))?;
    let mut png = Vec::new();
    image::DynamicImage::ImageRgb8(img)
        .write_to(
            &mut std::io::Cursor::new(&mut png),
            image::ImageFormat::Png,
        )
        .wrap_err("encode PNG")?;

    Ok(png)
}
