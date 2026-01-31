use eyre::{Result, WrapErr};
use ordered_float::OrderedFloat;
use poise::CreateReply;
use poise::serenity_prelude as serenity;
use std::time::Duration;
use time::{Duration as TimeDuration, OffsetDateTime, UtcOffset};

use crate::embeds::{build_mai_recent_embeds, build_mai_today_embed, build_mai_today_detail_embed, embed_base, format_level_with_internal, RecentRecordView};
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
        .backend_client
        .search_scores(&search)
        .await
        .wrap_err("search scores")?;

    if scores.is_empty() {
        ctx.send(CreateReply::default().embed(embed_base("No records found").description("No titles to match.")))
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
        ctx.send(CreateReply::default().ephemeral(true).embed(
            embed_base("No records found").description("No scores for this title."),
        ))
        .await?;
        return Ok(());
    }

    let mut embed = embed_base(&matched_title);
    let mut has_rows = false;

    for score in &matched_scores {
        has_rows = true;
        let achievement_percent = score.achievement_x10000.map(|x| x as f64 / 10000.0).unwrap_or(0.0);
        let level = format_level_with_internal(&score.level, score.internal_level);
        let rank = score.rank.as_deref().unwrap_or("N/A");
        let fc = score.fc.as_deref().unwrap_or("-");
        let sync = score.sync.as_deref().unwrap_or("-");

        let field_name = format!(
            "[{}] {} {}",
            score.chart_type,
            score.diff_category,
            level
        );

        let field_value = format!(
            "{:.4}% • {} • {} • {}",
            achievement_percent,
            rank,
            fc,
            sync
        );

        embed = embed.field(field_name, field_value, false);
    }

    let mut attachments = Vec::new();
    if let Some(score) = matched_scores.first() {
        if let Some(ref image_name) = score.image_name {
            embed = embed.thumbnail(format!("attachment://{image_name}"));
            match ctx.data().backend_client.get_cover(image_name).await {
                Ok(bytes) => {
                    attachments.push(serenity::CreateAttachment::bytes(bytes, image_name.clone()));
                }
                Err(e) => tracing::warn!("failed to fetch cover image {image_name}: {e:?}"),
            }
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

    let records: Vec<RecentRecordView> = recent
        .into_iter()
        .map(|record| RecentRecordView {
            track: record.track.map(|t| t as i64),
            played_at: record.played_at,
            title: record.title,
            chart_type: record.chart_type,
            diff_category: record.diff_category,
            level: record.level,
            internal_level: record.internal_level,
            rating_points: record.rating_points,
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
                rating_points: play.rating_points,
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

    let embeds = build_mai_rating_embeds(&ctx.data().backend_client).await?;

    ctx.send(CreateReply {
        embeds,
        ..Default::default()
    })
    .await?;

    Ok(())
}

async fn build_mai_rating_embeds(
    client: &crate::client::BackendClient,
) -> Result<Vec<serenity::builder::CreateEmbed>> {
    let scores = client.get_rated_scores().await?;

    #[derive(Debug, Clone)]
    struct RatedRow {
        bucket: String,
        title: String,
        chart_type: String,
        diff_category: String,
        level: String,
        internal_level: f32,
        achievement_percent: f64,
        rank: Option<String>,
        rating_points: u32,
    }

    let mut missing_data = 0usize;
    let mut out_rows = Vec::new();

    for score in scores {
        let Some(achievement_x10000) = score.achievement_x10000 else {
            continue;
        };
        let achievement_percent = achievement_x10000 as f64 / 10000.0;

        let Some(ref bucket) = score.bucket else {
            missing_data += 1;
            continue;
        };

        let Some(internal_level) = score.internal_level else {
            missing_data += 1;
            continue;
        };

        let Some(rating_points) = score.rating_points else {
            missing_data += 1;
            continue;
        };

        out_rows.push(RatedRow {
            bucket: bucket.clone(),
            title: score.title,
            chart_type: score.chart_type,
            diff_category: score.diff_category,
            level: score.level,
            internal_level,
            achievement_percent,
            rank: score.rank,
            rating_points,
        });
    }

    let mut new_rows = out_rows
        .iter()
        .filter(|r| r.bucket == "New")
        .cloned()
        .collect::<Vec<_>>();
    let mut old_rows = out_rows
        .iter()
        .filter(|r| r.bucket == "Old")
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
    if missing_data > 0 {
        summary = summary.field(
            "Notes",
            format!("missing song data: {missing_data}"),
            false,
        );
    }

    let new_embed = embed_base("NEW 15").description(list_desc(&new_rows));
    let old_embed = embed_base("OLD 35").description(list_desc(&old_rows));

    Ok(vec![summary, new_embed, old_embed])
}


