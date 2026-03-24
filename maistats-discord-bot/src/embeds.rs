use models::{ChartType, DifficultyCategory, FcStatus, ScoreRank, SyncStatus};
use poise::serenity_prelude as serenity;
use serenity::builder::{CreateEmbed, CreateEmbedFooter};

use crate::chart_links::linked_chart_label;
use crate::emoji::{MaimaiStatusEmojis, format_fc, format_rank, format_sync};

const EMBED_COLOR: u32 = 0x51BCF3;
const EMBED_COLOR_MAINTENANCE: u32 = 0xFFA500;

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

pub(crate) fn embed_startup_summary(registered_url_count: i64) -> CreateEmbed {
    embed_base("maistats-discord-bot ready").description(format!(
        "Startup complete.\n**Registered URLs**: {registered_url_count}"
    ))
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
    pub(crate) image_name: Option<String>,
    pub(crate) level: Option<String>,
    pub(crate) internal_level: Option<f32>,
    pub(crate) rating_points: Option<u32>,
    pub(crate) achievement_percent: Option<f64>,
    pub(crate) achievement_new_record: bool,
    pub(crate) rank: Option<ScoreRank>,
    pub(crate) fc: Option<FcStatus>,
    pub(crate) sync: Option<SyncStatus>,
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

fn format_recent_title(record: &RecentRecordView) -> String {
    record.title.clone()
}

fn format_recent_chart_line(record: &RecentRecordView) -> String {
    let level = record
        .level
        .as_deref()
        .map(|v| format_level_with_internal(v, record.internal_level))
        .unwrap_or_else(|| "N/A".to_string());

    match record.diff_category {
        Some(diff) => linked_chart_label(&record.title, record.chart_type, diff, &level),
        None => format!("[{}] Unknown {}", record.chart_type, level),
    }
}

fn format_recent_detail_lines(
    record: &RecentRecordView,
    status_emojis: &MaimaiStatusEmojis,
) -> String {
    let achievement = format_percent_f64(record.achievement_percent);
    let rank = format_rank(status_emojis, record.rank, "-");
    let fc = format_fc(status_emojis, record.fc, "-");
    let sync = format_sync(status_emojis, record.sync, "-");

    format!("{achievement} • {rank} • {fc} • {sync}")
}

fn format_recent_footer(record: &RecentRecordView) -> CreateEmbedFooter {
    let rating = record
        .rating_points
        .map(|v| v.to_string())
        .unwrap_or_else(|| "-".to_string());
    let played_at = record.played_at.as_deref().unwrap_or("-");

    CreateEmbedFooter::new(format!("Rating: {rating} • Played: {played_at}"))
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
    status_emojis: &MaimaiStatusEmojis,
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
        let mut desc = format!(
            "{}\n{}",
            format_recent_chart_line(record),
            format_recent_detail_lines(record, status_emojis)
        );
        if record.achievement_new_record {
            desc.push_str("\n**NEW RECORD**");
        }

        let mut embed = embed_base(&format_recent_title(record))
            .description(desc)
            .footer(format_recent_footer(record));
        if let Some(image_name) = record.image_name.as_deref() {
            embed = embed.thumbnail(format!("attachment://{image_name}"));
        }
        embed
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
) -> CreateEmbed {
    let mut e = embed_base(&format!("{}'s today", display_name));
    e = e
        .field("Window", format!("{} ~ {}", start, end), false)
        .field("Credits", credits.to_string(), true)
        .field("Tracks", tracks.to_string(), true)
        .field("New records", new_records.to_string(), true);
    e
}
