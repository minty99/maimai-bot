use eyre::{Result, WrapErr};
use poise::serenity_prelude as serenity;
use poise::CreateReply;
use std::time::Duration;
use time::{Duration as TimeDuration, OffsetDateTime, UtcOffset};

use crate::embeds::{
    build_mai_recent_embeds, build_mai_today_detail_embed, build_mai_today_embed, embed_base,
    format_level_with_internal, RecentRecordView,
};
use crate::BotData;

type Context<'a> = poise::Context<'a, BotData, Box<dyn std::error::Error + Send + Sync>>;
type Error = Box<dyn std::error::Error + Send + Sync>;

fn normalize_for_match(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect()
}

fn top_title_matches(query: &str, titles: &[String], limit: usize) -> Vec<String> {
    let query_norm = normalize_for_match(query);
    let mut scored: Vec<_> = titles
        .iter()
        .map(|title| {
            let title_norm = normalize_for_match(title);
            let score = strsim::jaro_winkler(&query_norm, &title_norm);
            (title.clone(), score)
        })
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.into_iter().take(limit).map(|(t, _)| t).collect()
}

/// Get song records by song title or key
#[poise::command(slash_command, rename = "mai-score")]
pub(crate) async fn mai_score(
    ctx: Context<'_>,
    #[description = "Song title to search for"] search: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let scores = ctx
        .data()
        .record_collector_client
        .search_scores(&search)
        .await
        .wrap_err("search scores")?;

    if scores.is_empty() {
        ctx.send(
            CreateReply::default()
                .embed(embed_base("No records found").description("No titles to match.")),
        )
        .await?;
        return Ok(());
    }

    let titles: Vec<String> = scores.iter().map(|s| s.title.clone()).collect();
    let search_norm = normalize_for_match(&search);
    let exact_title = titles
        .iter()
        .find(|t| normalize_for_match(t) == search_norm)
        .cloned();

    let matched_title = if let Some(exact) = exact_title {
        exact
    } else {
        let candidates = top_title_matches(&search, &titles, 5);
        if candidates.is_empty() {
            ctx.send(
                CreateReply::default()
                    .ephemeral(true)
                    .embed(embed_base("No records found").description("No titles to match.")),
            )
            .await?;
            return Ok(());
        }

        let uuid = ctx.id();
        let button_prefix = format!("{uuid}:score_pick:");

        let mut buttons = Vec::new();
        let mut lines = Vec::new();
        for (i, title) in candidates.iter().enumerate() {
            let custom_id = format!("{button_prefix}{i}");
            buttons.push(
                serenity::CreateButton::new(custom_id)
                    .style(serenity::ButtonStyle::Secondary)
                    .label(format!("{}", i + 1)),
            );
            lines.push(format!("`{}` {}", i + 1, title));
        }

        let reply = ctx
            .send(
                CreateReply::default()
                    .embed(
                        embed_base("No exact match")
                            .description(format!("Query: `{search}`\n\n{}", lines.join("\n"))),
                    )
                    .components(vec![serenity::CreateActionRow::Buttons(buttons)]),
            )
            .await?;

        let interaction = serenity::ComponentInteractionCollector::new(ctx)
            .author_id(ctx.author().id)
            .channel_id(ctx.channel_id())
            .timeout(Duration::from_secs(60))
            .filter({
                let button_prefix = button_prefix.clone();
                move |mci| mci.data.custom_id.starts_with(&button_prefix)
            })
            .await;

        let Some(mci) = interaction else {
            if let Ok(msg) = reply.message().await {
                let mut msg = msg.into_owned();
                msg.edit(
                    ctx,
                    serenity::EditMessage::new()
                        .embed(embed_base("No exact match").description(
                            "Timed out. Re-run `/mai-score <title>` with one of the suggested titles.",
                        ))
                        .components(Vec::new()),
                )
                .await?;
            }
            return Ok(());
        };

        mci.create_response(ctx, serenity::CreateInteractionResponse::Acknowledge)
            .await?;

        let idx = mci
            .data
            .custom_id
            .strip_prefix(&button_prefix)
            .and_then(|s| s.parse::<usize>().ok());

        let Some(idx) = idx else {
            return Ok(());
        };
        if idx >= candidates.len() {
            return Ok(());
        }

        if let Ok(msg) = reply.message().await {
            let msg = msg.into_owned();
            let _ = msg.delete(ctx).await;
        }

        candidates[idx].clone()
    };

    let matched_scores: Vec<_> = scores
        .iter()
        .filter(|s| s.title == matched_title)
        .cloned()
        .collect();

    if matched_scores.is_empty() {
        ctx.send(
            CreateReply::default()
                .ephemeral(true)
                .embed(embed_base("No records found").description("No scores for this title.")),
        )
        .await?;
        return Ok(());
    }

    let mut embed = embed_base(&matched_title);
    let mut has_rows = false;
    let mut first_image_name = None::<String>;

    for score in &matched_scores {
        has_rows = true;
        let metadata = fetch_song_metadata(
            &ctx.data().song_info_client,
            &score.title,
            score.chart_type,
            score.diff_category,
        )
        .await;

        let achievement_percent = score
            .achievement_x10000
            .map(|x| x as f64 / 10000.0)
            .unwrap_or(0.0);
        let level = metadata
            .as_ref()
            .and_then(|m| m.level.as_deref())
            .unwrap_or("N/A");
        let level =
            format_level_with_internal(level, metadata.as_ref().and_then(|m| m.internal_level));
        let rank = score.rank.map(|r| r.as_str()).unwrap_or("N/A");
        let fc = score.fc.map(|v| v.as_str()).unwrap_or("-");
        let sync = score.sync.map(|v| v.as_str()).unwrap_or("-");

        if first_image_name.is_none() {
            first_image_name = metadata.and_then(|m| m.image_name);
        }

        let field_name = format!("[{}] {} {}", score.chart_type, score.diff_category, level);

        let field_value = format!("{:.4}% • {} • {} • {}", achievement_percent, rank, fc, sync);

        embed = embed.field(field_name, field_value, false);
    }

    let mut attachments = Vec::new();
    if let Some(ref image_name) = first_image_name {
        embed = embed.thumbnail(format!("attachment://{image_name}"));
        match ctx.data().song_info_client.get_cover(image_name).await {
            Ok(bytes) => {
                attachments.push(serenity::CreateAttachment::bytes(bytes, image_name.clone()));
            }
            Err(e) => tracing::warn!("failed to fetch cover image {image_name}: {e:?}"),
        }
    }

    ctx.send(CreateReply {
        embeds: vec![embed],
        attachments,
        ephemeral: Some(!has_rows),
        ..Default::default()
    })
    .await?;

    Ok(())
}

/// Get most recent credit records
#[poise::command(slash_command, rename = "mai-recent")]
pub(crate) async fn mai_recent(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;

    let play_records = ctx
        .data()
        .record_collector_client
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

    let mut records = Vec::with_capacity(recent.len());
    for record in recent {
        let metadata = match record.diff_category {
            Some(diff_category) => {
                fetch_song_metadata(
                    &ctx.data().song_info_client,
                    &record.title,
                    record.chart_type,
                    diff_category,
                )
                .await
            }
            None => None,
        };
        let internal_level = metadata.as_ref().and_then(|m| m.internal_level);
        let rating_points = match (internal_level, record.achievement_x10000) {
            (Some(internal), Some(achievement_x10000)) => Some(chart_rating_points(
                internal as f64,
                achievement_x10000 as f64 / 10000.0,
                is_ap_like(record.fc.as_ref()),
            )),
            _ => None,
        };
        records.push(RecentRecordView {
            track: record.track.map(|t| t as i64),
            played_at: record.played_at,
            title: record.title,
            chart_type: record.chart_type,
            diff_category: record.diff_category,
            level: metadata.as_ref().and_then(|m| m.level.clone()),
            internal_level,
            rating_points,
            achievement_percent: record.achievement_x10000.map(|x| x as f64 / 10000.0),
            achievement_new_record: record.achievement_new_record.unwrap_or(0) != 0,
            first_play: record.first_play.unwrap_or(0) != 0,
            rank: record.score_rank,
        });
    }

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

    let plays = ctx
        .data()
        .record_collector_client
        .get_today(&today_str)
        .await?;

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
    #[description = "Date in YYYY-MM-DD (default: today JST, day boundary 04:00)"] date: Option<
        String,
    >,
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
        time::Date::from_calendar_date(
            year,
            time::Month::try_from(month).wrap_err("parse month")?,
            day,
        )
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

    let plays = ctx
        .data()
        .record_collector_client
        .get_today(&day_key)
        .await?;

    let mut rows = Vec::with_capacity(plays.len());
    for play in plays {
        let metadata = match play.diff_category {
            Some(diff_category) => {
                fetch_song_metadata(
                    &ctx.data().song_info_client,
                    &play.title,
                    play.chart_type,
                    diff_category,
                )
                .await
            }
            None => None,
        };
        let internal_level = metadata.as_ref().and_then(|m| m.internal_level);
        let rating_points = match (internal_level, play.achievement_x10000) {
            (Some(internal), Some(achievement_x10000)) => Some(chart_rating_points(
                internal as f64,
                achievement_x10000 as f64 / 10000.0,
                is_ap_like(play.fc.as_ref()),
            )),
            _ => None,
        };

        rows.push(crate::embeds::TodayDetailRowView {
            title: play.title,
            chart_type: play.chart_type,
            achievement_percent: play.achievement_x10000.map(|x| x as f64 / 10000.0),
            rating_points,
            achievement_new_record: play.achievement_new_record.unwrap_or(0) != 0,
            first_play: play.first_play.unwrap_or(0) != 0,
        });
    }

    rows.sort_by_key(|r| std::cmp::Reverse(r.rating_points.unwrap_or(0)));

    let display_name = "Player";
    let embed = build_mai_today_detail_embed(display_name, &day_key, &start, &end, &rows);

    if let Ok(dm) = ctx
        .author()
        .create_dm_channel(&ctx.serenity_context().http)
        .await
    {
        dm.send_message(
            &ctx.serenity_context().http,
            serenity::CreateMessage::new().embed(embed.clone()),
        )
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

    ctx.send(
        CreateReply::default().ephemeral(true).embed(
            embed_base("Temporarily Disabled").description(
                "`/mai-rating` is temporarily disabled.\nIt will be reimplemented with a different approach.",
            ),
        ),
    )
    .await?;

    Ok(())
}

async fn fetch_song_metadata(
    song_info_client: &crate::client::SongInfoClient,
    title: &str,
    chart_type: models::ChartType,
    diff_category: models::DifficultyCategory,
) -> Option<crate::client::SongMetadata> {
    match song_info_client
        .get_song_metadata(title, chart_type.as_str(), diff_category.as_str())
        .await
    {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(
                "failed to fetch song metadata for {title} [{chart_type} {diff_category}]: {e:#}"
            );
            None
        }
    }
}

fn is_ap_like(fc: Option<&models::FcStatus>) -> bool {
    matches!(
        fc,
        Some(&models::FcStatus::Ap) | Some(&models::FcStatus::ApPlus)
    )
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
