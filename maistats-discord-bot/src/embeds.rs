use models::{ChartType, DifficultyCategory, ScoreRank};
use poise::serenity_prelude as serenity;
use serenity::builder::CreateEmbed;

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

pub(crate) fn embed_registration_confirmation(player_name: &str, url: &str) -> CreateEmbed {
    embed_base("Record collector registered").description(format!(
        "**Player**: {player_name}\n**Record collector**: {url}"
    ))
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
        let rank = record.rank.map(|r| r.as_str()).unwrap_or("N/A");
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
        if record.achievement_new_record {
            desc.push_str("\n[NEW RECORD]");
        }

        let mut embed = embed_base(&track).description(desc);
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

fn format_track_label(track: Option<i64>) -> String {
    track
        .map(|t| format!("TRACK {t:02}"))
        .unwrap_or_else(|| "TRACK ??".to_string())
}
