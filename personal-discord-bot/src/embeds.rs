use models::{ChartType, DifficultyCategory};
use poise::serenity_prelude as serenity;
use serenity::builder::CreateEmbed;

const EMBED_COLOR: u32 = 0x51BCF3;
const EMBED_COLOR_MAINTENANCE: u32 = 0xFFA500;
const EMBED_COLOR_WARNING: u32 = 0xFFD700;

pub(crate) fn embed_base(title: &str) -> CreateEmbed {
    let mut e = CreateEmbed::new();
    e = e.title(title).color(EMBED_COLOR);
    e
}

pub(crate) fn embed_maintenance() -> CreateEmbed {
    CreateEmbed::new()
        .title("🔧 Maintenance Mode")
        .description(
            "Bot started successfully! maimai DX NET is in scheduled maintenance \
            (04:00-07:00). Normal monitoring will resume after maintenance.",
        )
        .color(EMBED_COLOR_MAINTENANCE)
}

pub(crate) fn embed_record_collector_unavailable() -> CreateEmbed {
    CreateEmbed::new()
        .title("⚠️ Record Collector Starting Up")
        .description(
            "Bot started successfully! Couldn't fetch player data right now. \
            I'll monitor for new plays once the record collector is ready.",
        )
        .color(EMBED_COLOR_WARNING)
}

#[derive(Debug, Clone)]
pub(crate) struct RecentOptionalFields {
    pub(crate) rating: Option<String>,
    pub(crate) play_count: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct RecentRecordView {
    pub(crate) track: Option<i64>,
    pub(crate) played_at: Option<String>,
    pub(crate) title: String,
    pub(crate) chart_type: ChartType,
    pub(crate) diff_category: Option<DifficultyCategory>,
    pub(crate) level: Option<String>,
    pub(crate) internal_level: Option<f32>,
    pub(crate) rating_points: Option<u32>,
    pub(crate) achievement_percent: Option<f64>,
    pub(crate) achievement_new_record: bool,
    pub(crate) first_play: bool,
    pub(crate) rank: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct TodayDetailRowView {
    pub(crate) title: String,
    pub(crate) chart_type: ChartType,
    pub(crate) achievement_percent: Option<f64>,
    pub(crate) rating_points: Option<u32>,
    pub(crate) achievement_new_record: bool,
    pub(crate) first_play: bool,
}

pub(crate) fn format_level_with_internal(level: &str, internal_level: Option<f32>) -> String {
    if level == "N/A" {
        return level.to_string();
    }
    match internal_level {
        Some(v) => format!("{level} ({v:.1})"),
        None => level.to_string(),
    }
}

fn format_rating_points_suffix(rating_points: Option<u32>) -> String {
    match rating_points {
        Some(v) => format!(" • {v}pt"),
        None => String::new(),
    }
}

fn format_percent_f64(value: Option<f64>) -> String {
    match value {
        Some(v) => format!("{:.4}%", v),
        None => "N/A".to_string(),
    }
}

pub(crate) fn build_mai_recent_embeds(
    display_name: &str,
    records: &[RecentRecordView],
    optional_fields: Option<&RecentOptionalFields>,
) -> Vec<CreateEmbed> {
    let mut embeds = Vec::new();

    if let Some(fields) = optional_fields {
        let started_at = records
            .iter()
            .find(|r| r.track == Some(1))
            .and_then(|r| r.played_at.as_deref())
            .or_else(|| records.iter().find_map(|r| r.played_at.as_deref()));

        let mut summary = embed_base(&format!("{display_name}'s latest credit"));
        if let Some(v) = fields.rating.as_deref() {
            summary = summary.field("Rating", v, true);
        }
        if let Some(v) = fields.play_count.as_deref() {
            summary = summary.field("Play count", v, true);
        }
        if let Some(v) = started_at {
            summary = summary.field("Credit started at", v, false);
        }

        embeds.push(summary);
    }

    embeds.extend(records.iter().map(|record| {
        let track = format_track_label(record.track);
        let achv = format_percent_f64(record.achievement_percent);
        let rank = record
            .rank
            .as_deref()
            .map(normalize_playlog_rank)
            .unwrap_or("N/A");
        let diff = record
            .diff_category
            .map(|d| d.as_str())
            .unwrap_or("Unknown");
        let level = record.level.as_deref().unwrap_or("N/A");
        let level = format_level_with_internal(level, record.internal_level);
        let rating = format_rating_points_suffix(record.rating_points);
        let mut desc = format!(
            "**{}**\n[{}] {diff} {level} — {achv} • {rank}{rating}",
            record.title, record.chart_type
        );
        if record.first_play {
            desc.push_str("\n[FIRST PLAY]");
        } else if record.achievement_new_record {
            desc.push_str("\n[NEW RECORD]");
        }
        embed_base(&track).description(desc)
    }));

    embeds
}

pub(crate) fn build_mai_today_embed(
    display_name: &str,
    start: &str,
    end: &str,
    credits: i64,
    tracks: i64,
    new_records: i64,
    first_plays: i64,
) -> CreateEmbed {
    let mut e = embed_base(&format!("{}'s today", display_name));
    e = e
        .field("Window", format!("{} ~ {}", start, end), false)
        .field("Credits", credits.to_string(), true)
        .field("Tracks", tracks.to_string(), true)
        .field("New records", new_records.to_string(), true)
        .field("First plays", first_plays.to_string(), true);
    e
}

pub(crate) fn build_mai_today_detail_embed(
    display_name: &str,
    day_key: &str,
    start: &str,
    end: &str,
    rows: &[TodayDetailRowView],
) -> CreateEmbed {
    let mut desc = String::new();
    let total = rows.len();

    for (idx, row) in rows.iter().enumerate() {
        let achv = format_percent_f64(row.achievement_percent);
        let mut line = format!("- **{}** [{}] — {}", row.title, row.chart_type, achv);
        if let Some(pt) = row.rating_points {
            line.push_str(&format!(" • {pt}pt"));
        }
        if row.first_play {
            line.push_str(" [FIRST PLAY]");
        } else if row.achievement_new_record {
            line.push_str(" [NEW RECORD]");
        }
        line.push('\n');

        if desc.len().saturating_add(line.len()) > 3900 {
            desc.push_str(&format!("... (truncated; showing {}/{total})\n", idx));
            break;
        }
        desc.push_str(&line);
    }

    if desc.trim().is_empty() {
        desc = "No playlogs found for this day.".to_string();
    }

    embed_base(&format!("{display_name}'s plays on {day_key}"))
        .field("Window", format!("{start} ~ {end}"), false)
        .description(desc)
}

fn format_track_label(track: Option<i64>) -> String {
    track
        .map(|t| format!("TRACK {t:02}"))
        .unwrap_or_else(|| "TRACK ??".to_string())
}

fn normalize_playlog_rank(rank: &str) -> &str {
    match rank {
        "SSSPLUS" => "SSS+",
        "SSPLUS" => "SS+",
        "SPLUS" => "S+",
        _ => rank,
    }
}
