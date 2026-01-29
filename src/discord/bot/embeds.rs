use poise::serenity_prelude as serenity;
use serenity::builder::CreateEmbed;

use crate::db::{format_chart_type, format_percent_f64};
use crate::maimai::models::{ParsedPlayRecord, ParsedPlayerData};
use crate::maimai::rating::{chart_rating_points, is_ap_like};
use crate::song_data::SongDataIndex;

const EMBED_COLOR: u32 = 0x51BCF3;

pub(crate) fn embed_base(title: &str) -> CreateEmbed {
    let mut e = CreateEmbed::new();
    e = e.title(title).color(EMBED_COLOR);
    e
}

pub(crate) fn format_delta(current: u32, previous: Option<u32>) -> String {
    let Some(previous) = previous else {
        return format!("{current}");
    };
    let delta = current as i64 - previous as i64;
    if delta > 0 {
        format!("{current} (+{delta})")
    } else if delta < 0 {
        format!("{current} ({delta})")
    } else {
        format!("{current} (+0)")
    }
}

pub(crate) fn embed_startup(player: &ParsedPlayerData) -> CreateEmbed {
    let play_count = format!(
        "{} ({})",
        player.total_play_count, player.current_version_play_count
    );
    embed_base("maimai-bot started")
        .field("User", &player.user_name, true)
        .field("Rating", player.rating.to_string(), true)
        .field("Play count", play_count, true)
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
    pub(crate) chart_type: String,
    pub(crate) diff_category: Option<String>,
    pub(crate) level: Option<String>,
    pub(crate) internal_level: Option<f32>,
    pub(crate) rating_points: Option<u32>,
    pub(crate) achievement_percent: Option<f64>,
    pub(crate) achievement_new_record: bool,
    pub(crate) first_play: bool,
    pub(crate) rank: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ScoreRowView {
    pub(crate) chart_type: String,
    pub(crate) diff_category: String,
    pub(crate) level: String,
    pub(crate) internal_level: Option<f32>,
    pub(crate) rating_points: Option<u32>,
    pub(crate) achievement_percent: Option<f64>,
    pub(crate) rank: Option<String>,
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

pub(crate) fn build_mai_score_embed(
    display_name: &str,
    title: &str,
    entries: &[ScoreRowView],
) -> CreateEmbed {
    let mut desc = String::new();
    desc.push_str(&format!("**{}**\n\n", title));

    for entry in entries {
        let achv = format_percent_f64(entry.achievement_percent);
        let rank = entry.rank.as_deref().unwrap_or("N/A");
        let level = format_level_with_internal(&entry.level, entry.internal_level);
        let rating = format_rating_points_suffix(entry.rating_points);
        desc.push_str(&format!(
            "- [{}] {} {} — {} • {}{}\n",
            entry.chart_type, entry.diff_category, level, achv, rank, rating
        ));
    }

    embed_base(&format!("{}'s scores", display_name)).description(desc)
}

pub(crate) fn build_mai_recent_embeds(
    display_name: &str,
    records: &[RecentRecordView],
    optional_fields: Option<&RecentOptionalFields>,
    song_data: Option<&SongDataIndex>,
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
        let diff = record.diff_category.as_deref().unwrap_or("Unknown");
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
        let mut embed = embed_base(&track).description(desc);

        if let Some(idx) = song_data
            && let Some(image_name) = idx.image_name(&record.title)
        {
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

pub(crate) fn rating_points_for_credit_entry(
    song_data: Option<&SongDataIndex>,
    entry: &ParsedPlayRecord,
) -> Option<u32> {
    let song_data = song_data?;
    let diff_category = entry.diff_category?;
    let achievement = entry.achievement_percent? as f64;

    let chart_type = format_chart_type(entry.chart_type);
    let internal_level =
        song_data.internal_level(&entry.title, chart_type, diff_category.as_str())?;

    let ap = is_ap_like(entry.fc.map(|v| v.as_str()));
    Some(chart_rating_points(internal_level as f64, achievement, ap))
}
