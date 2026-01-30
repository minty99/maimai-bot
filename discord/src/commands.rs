use eyre::{Result, WrapErr};
use models::SongDataIndex;
use ordered_float::OrderedFloat;
use poise::CreateReply;
use poise::serenity_prelude as serenity;
use time::{Duration as TimeDuration, OffsetDateTime, UtcOffset};

use crate::embeds::{build_mai_recent_embeds, build_mai_today_embed, build_mai_today_detail_embed, embed_base, format_level_with_internal, RecentRecordView};
use crate::BotData;

type Context<'a> = poise::Context<'a, BotData, Box<dyn std::error::Error + Send + Sync>>;
type Error = Box<dyn std::error::Error + Send + Sync>;

/// Get most recent credit records
#[poise::command(slash_command, rename = "mai-recent")]
pub(crate) async fn mai_recent(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;

    let play_records = ctx
        .data()
        .backend_client
        .get_recent(50)
        .await
        .wrap_err("fetch recent plays")?;

    if play_records.is_empty() {
        ctx.send(CreateReply::default().embed(embed_base("No recent records found")))
            .await?;
        return Ok(());
    }

    // Extract tracks to find latest credit (first TRACK 01)
    let tracks: Vec<Option<i64>> = play_records
        .iter()
        .map(|r| r.track.map(|t| t as i64))
        .collect();
    let take = latest_credit_len(&tracks);

    // Take the latest credit and reverse to display TRACK 01 first
    let mut recent = play_records.into_iter().take(take).collect::<Vec<_>>();
    recent.reverse();

    // Convert PlayRecord to RecentRecordView
    let records: Vec<RecentRecordView> = recent
        .into_iter()
        .map(|record| RecentRecordView {
            track: record.track.map(|t| t as i64),
            played_at: record.played_at,
            title: record.title,
            chart_type: record.chart_type,
            diff_category: record.diff_category,
            level: record.level,
            internal_level: None, // Backend doesn't provide internal level yet
            rating_points: None,  // Backend doesn't provide rating points yet
            achievement_percent: record.achievement_x10000.map(|x| x as f64 / 10000.0),
            achievement_new_record: record.achievement_new_record.unwrap_or(0) != 0,
            first_play: record.first_play.unwrap_or(0) != 0,
            rank: record.score_rank,
        })
        .collect();

    let embeds = build_mai_recent_embeds("Player", &records, None);

    ctx.send(CreateReply {
        embeds,
        ..Default::default()
    })
    .await?;

    Ok(())
}

/// Find the length of the latest credit (up to and including first TRACK 01)
fn latest_credit_len(tracks: &[Option<i64>]) -> usize {
    match tracks.iter().position(|t| *t == Some(1)) {
        Some(idx) => idx + 1,
        None => tracks.len().min(4),
    }
}

/// Show today's play summary (day boundary: 04:00 JST)
#[poise::command(slash_command, rename = "mai-today")]
pub(crate) async fn mai_today(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;

    let offset = UtcOffset::from_hms(9, 0, 0).unwrap_or(UtcOffset::UTC);
    let now_jst = OffsetDateTime::now_utc().to_offset(offset);

    let day_date = if now_jst.hour() < 4 {
        (now_jst - TimeDuration::days(1)).date()
    } else {
        now_jst.date()
    };
    let end_date = day_date + TimeDuration::days(1);

    let today_str = format!(
        "{:04}-{:02}-{:02}",
        day_date.year(),
        u8::from(day_date.month()),
        day_date.day()
    );

    let plays = ctx.data().backend_client.get_today(&today_str).await?;

    let tracks = plays.len() as i64;
    let credits = plays
        .iter()
        .filter_map(|p| p.credit_play_count)
        .collect::<std::collections::HashSet<_>>()
        .len() as i64;
    let first_plays = plays
        .iter()
        .filter(|p| p.first_play.unwrap_or(0) != 0)
        .count() as i64;
    let new_record_flags = plays
        .iter()
        .filter(|p| p.achievement_new_record.unwrap_or(0) != 0)
        .count() as i64;
    let new_records_true = (new_record_flags - first_plays).max(0);

    let start = format!("{} 04:00", today_str);
    let end = format!(
        "{:04}-{:02}-{:02} 04:00",
        end_date.year(),
        u8::from(end_date.month()),
        end_date.day()
    );

    let display_name = "Player";

    let embed = build_mai_today_embed(
        display_name,
        &start,
        &end,
        credits,
        tracks,
        new_records_true,
        first_plays,
    );

    ctx.send(CreateReply::default().embed(embed)).await?;
    Ok(())
}

/// Show today's (or a specified day's) play detail as a DM (day boundary: 04:00 JST)
#[poise::command(slash_command, rename = "mai-today-detail")]
pub(crate) async fn mai_today_detail(
    ctx: Context<'_>,
    #[description = "Date in YYYY-MM-DD (default: today JST, day boundary 04:00)"] date: Option<String>,
) -> Result<(), Error> {
    ctx.defer().await?;

    let offset = UtcOffset::from_hms(9, 0, 0).unwrap_or(UtcOffset::UTC);
    
    let day_date = if let Some(date_str) = date.as_deref() {
        let key = date_str.trim().replace('-', "/");
        let parts = key.split('/').collect::<Vec<_>>();
        if parts.len() != 3 {
            return Err("date must be YYYY-MM-DD".into());
        }
        let year = parts[0].parse::<i32>().wrap_err("parse year")?;
        let month = parts[1].parse::<u8>().wrap_err("parse month")?;
        let day = parts[2].parse::<u8>().wrap_err("parse day")?;
        time::Date::from_calendar_date(year, time::Month::try_from(month).wrap_err("parse month")?, day)
            .wrap_err("parse date")?
    } else {
        let now_jst = OffsetDateTime::now_utc().to_offset(offset);
        if now_jst.hour() < 4 {
            (now_jst - TimeDuration::days(1)).date()
        } else {
            now_jst.date()
        }
    };
    
    let end_date = day_date + TimeDuration::days(1);

    let day_key = format!(
        "{:04}-{:02}-{:02}",
        day_date.year(),
        u8::from(day_date.month()),
        day_date.day()
    );
    let end_key = format!(
        "{:04}-{:02}-{:02}",
        end_date.year(),
        u8::from(end_date.month()),
        end_date.day()
    );

    let start = format!("{} 04:00", day_key);
    let end = format!("{} 04:00", end_key);

    let plays = ctx.data().backend_client.get_today(&day_key).await?;

    let mut rows: Vec<_> = plays
        .into_iter()
        .map(|play| {
            let achievement_percent = play.achievement_x10000.map(|x| x as f64 / 10000.0);
            crate::embeds::TodayDetailRowView {
                title: play.title,
                chart_type: play.chart_type,
                achievement_percent,
                rating_points: None,
                achievement_new_record: play.achievement_new_record.unwrap_or(0) != 0,
                first_play: play.first_play.unwrap_or(0) != 0,
            }
        })
        .collect();

    rows.sort_by_key(|r| std::cmp::Reverse(r.rating_points.unwrap_or(0)));

    let display_name = "Player";
    let embed = build_mai_today_detail_embed(
        display_name,
        &day_key,
        &start,
        &end,
        &rows,
    );

    if let Ok(dm) = ctx.author().create_dm_channel(&ctx.serenity_context().http).await {
        dm.send_message(&ctx.serenity_context().http, serenity::CreateMessage::new().embed(embed.clone()))
            .await
            .wrap_err("send DM")?;
    }

    ctx.send(CreateReply::default().ephemeral(true).embed(
        embed_base("Sent").description(format!("Sent a DM with play details for `{day_key}`.")),
    ))
    .await?;

    Ok(())
}

/// Show rating breakdown (CiRCLE baseline)
#[poise::command(slash_command, rename = "mai-rating")]
pub(crate) async fn mai_rating(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;

    let song_data = ctx.data().song_data.as_ref();
    let embeds = build_mai_rating_embeds(&ctx.data().backend_client, song_data).await?;

    ctx.send(CreateReply {
        embeds,
        ..Default::default()
    })
    .await?;

    Ok(())
}

async fn build_mai_rating_embeds(
    client: &crate::client::BackendClient,
    song_data: Option<&SongDataIndex>,
) -> Result<Vec<serenity::builder::CreateEmbed>> {
    let Some(song_data) = song_data else {
        return Ok(vec![
            embed_base("song data not loaded")
                .description("Cannot compute rating without song metadata."),
        ]);
    };

    let records = client.get_recent(1000).await?;

    #[derive(Debug, Clone)]
    struct RatedRow {
        bucket: models::SongBucket,
        title: String,
        chart_type: String,
        diff_category: String,
        level: String,
        internal_level: f32,
        achievement_percent: f64,
        rank: Option<String>,
        rating_points: u32,
    }

    let mut missing_internal = 0usize;
    let mut missing_bucket = 0usize;
    let mut out_rows = Vec::new();

    for record in records {
        // Only process records with achievement
        let Some(achievement_x10000) = record.achievement_x10000 else {
            continue;
        };
        let achievement_percent = achievement_x10000 as f64 / 10000.0;

        let Some(bucket) = song_data.bucket(&record.title) else {
            missing_bucket += 1;
            continue;
        };

        let Some(internal_level) = song_data.internal_level(
            &record.title,
            &record.chart_type,
            record.diff_category.as_deref().unwrap_or(""),
        ) else {
            missing_internal += 1;
            continue;
        };

        let ap = is_ap_like(record.fc.as_deref());
        let rating_points = chart_rating_points(internal_level as f64, achievement_percent, ap);

        out_rows.push(RatedRow {
            bucket,
            title: record.title,
            chart_type: record.chart_type,
            diff_category: record.diff_category.unwrap_or_default(),
            level: record.level.unwrap_or_default(),
            internal_level,
            achievement_percent,
            rank: record.score_rank,
            rating_points,
        });
    }

    let mut new_rows = out_rows
        .iter()
        .filter(|r| r.bucket == models::SongBucket::New)
        .cloned()
        .collect::<Vec<_>>();
    let mut old_rows = out_rows
        .iter()
        .filter(|r| r.bucket == models::SongBucket::Old)
        .cloned()
        .collect::<Vec<_>>();

    new_rows
        .sort_by_key(|r| std::cmp::Reverse((r.rating_points, OrderedFloat(r.achievement_percent))));
    old_rows
        .sort_by_key(|r| std::cmp::Reverse((r.rating_points, OrderedFloat(r.achievement_percent))));

    let new_rows = new_rows.into_iter().take(15).collect::<Vec<_>>();
    let old_rows = old_rows.into_iter().take(35).collect::<Vec<_>>();

    let new_sum = new_rows.iter().map(|r| r.rating_points).sum::<u32>();
    let old_sum = old_rows.iter().map(|r| r.rating_points).sum::<u32>();
    let total = new_sum.saturating_add(old_sum);

    fn list_desc(rows: &[RatedRow]) -> String {
        let mut out = String::new();
        for (idx, r) in rows.iter().enumerate() {
            let rank = r.rank.as_deref().unwrap_or("N/A");
            let level = format_level_with_internal(&r.level, Some(r.internal_level));
            out.push_str(&format!(
                "- [{}] `{:>3}pt` {} [{}] {} {} — {:.4}% • {}\n",
                idx + 1,
                r.rating_points,
                r.title,
                r.chart_type,
                r.diff_category,
                level,
                r.achievement_percent,
                rank
            ));
        }
        out
    }

    let mut summary = embed_base("Rating")
        .field("Computed", total.to_string(), true)
        .field("NEW 15", new_sum.to_string(), true)
        .field("OLD 35", old_sum.to_string(), true);
    if missing_internal > 0 || missing_bucket > 0 {
        summary = summary.field(
            "Notes",
            format!(
                "missing internal level: {missing_internal}\nmissing song version: {missing_bucket}"
            ),
            false,
        );
    }

    let new_embed = embed_base("NEW 15").description(list_desc(&new_rows));
    let old_embed = embed_base("OLD 35").description(list_desc(&old_rows));

    Ok(vec![summary, new_embed, old_embed])
}

fn is_ap_like(fc: Option<&str>) -> bool {
    matches!(fc, Some("AP") | Some("AP+"))
}

fn coefficient_for_achievement(achievement_percent: f64) -> f64 {
    const ACHIEVEMENT_CAP: f64 = 100.5;
    let a = achievement_percent.min(ACHIEVEMENT_CAP);

    if a >= 100.5 {
        22.4
    } else if a >= 100.4999 {
        22.2
    } else if a >= 100.0 {
        21.6
    } else if a >= 99.9999 {
        21.4
    } else if a >= 99.5 {
        21.1
    } else if a >= 99.0 {
        20.8
    } else if a >= 98.9999 {
        20.6
    } else if a >= 98.0 {
        20.3
    } else if a >= 97.0 {
        20.0
    } else if a >= 96.9999 {
        17.6
    } else if a >= 94.0 {
        16.8
    } else if a >= 90.0 {
        15.2
    } else if a >= 80.0 {
        13.6
    } else if a >= 79.9999 {
        12.8
    } else if a >= 75.0 {
        12.0
    } else if a >= 70.0 {
        11.2
    } else if a >= 60.0 {
        9.6
    } else if a >= 50.0 {
        8.0
    } else if a >= 40.0 {
        6.4
    } else if a >= 30.0 {
        4.8
    } else if a >= 20.0 {
        3.2
    } else if a >= 10.0 {
        1.6
    } else {
        0.0
    }
}

fn chart_rating_points(internal_level: f64, achievement_percent: f64, ap_bonus: bool) -> u32 {
    const ACHIEVEMENT_CAP: f64 = 100.5;
    let coef = coefficient_for_achievement(achievement_percent);
    let ach = achievement_percent.min(ACHIEVEMENT_CAP);
    let base = ((coef * internal_level * ach) / 100.0).floor();
    let base = if base.is_finite() && base > 0.0 {
        base as u32
    } else {
        0
    };
    if ap_bonus {
        base.saturating_add(1)
    } else {
        base
    }
}
