use eyre::{Result, WrapErr};
use poise::CreateReply;
use poise::serenity_prelude as serenity;
use std::time::Duration;
use tracing::warn;

use crate::discord::mai_commands;

use super::dm::send_embed_dm;
use super::embeds::embed_base;
use super::refresh::refresh_from_network_if_needed;
use super::types::{BotData, Context, Error};
use super::util::{normalize_for_match, top_title_matches};

async fn display_user_name(ctx: &poise::Context<'_, BotData, Error>) -> String {
    let name = ctx.data().maimai_user_name.read().await.clone();
    if name.trim().is_empty() {
        ctx.author().name.clone()
    } else {
        name
    }
}

/// Get song records by song title or key
#[poise::command(slash_command, rename = "mai-score")]
pub(crate) async fn mai_score(
    ctx: Context<'_>,
    #[description = "Song title to search for"] search: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    if let Err(e) = refresh_from_network_if_needed(ctx.data()).await {
        warn!("mai-score: refresh failed; continuing with DB: {e:#}");
    }

    let titles = mai_commands::fetch_score_titles(&ctx.data().db).await?;

    if titles.is_empty() {
        ctx.send(CreateReply::default().embed(mai_commands::embed_no_scores_found()))
            .await?;
        return Ok(());
    }

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

    let display_name = display_user_name(&ctx).await;
    let (mut embed, has_rows) = mai_commands::build_mai_score_embed_for_title(
        &ctx.data().db,
        ctx.data().song_data.as_deref(),
        &display_name,
        &matched_title,
    )
    .await?;

    let mut attachments = Vec::new();
    if let Some(idx) = ctx.data().song_data.as_deref()
        && let Some(image_name) = idx.image_name(&matched_title)
    {
        embed = embed.thumbnail(format!("attachment://{image_name}"));
        let path = format!("fetched_data/img/cover-m/{image_name}");
        match serenity::CreateAttachment::path(&path).await {
            Ok(att) => attachments.push(att),
            Err(e) => warn!("failed to attach cover image {path}: {e:?}"),
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

    if let Err(e) = refresh_from_network_if_needed(ctx.data()).await {
        warn!("mai-recent: refresh failed; continuing with DB: {e:#}");
    }

    let display_name = display_user_name(&ctx).await;

    let embeds = mai_commands::build_mai_recent_embeds_for_latest_credit(
        &ctx.data().db,
        ctx.data().song_data.as_deref(),
        &display_name,
        None,
    )
    .await?;

    ctx.send(CreateReply {
        embeds,
        ..Default::default()
    })
    .await?;

    Ok(())
}

/// Show rating breakdown (CiRCLE baseline)
#[poise::command(slash_command, rename = "mai-rating")]
pub(crate) async fn mai_rating(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;

    let display_name = display_user_name(&ctx).await;
    let embeds = mai_commands::build_mai_rating_embeds(
        &ctx.data().db,
        ctx.data().song_data.as_deref(),
        &display_name,
    )
    .await?;

    ctx.send(CreateReply {
        embeds,
        ..Default::default()
    })
    .await?;

    Ok(())
}

/// Show today's play summary (day boundary: 04:00 JST)
#[poise::command(slash_command, rename = "mai-today")]
pub(crate) async fn mai_today(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;

    let display_name = display_user_name(&ctx).await;
    let embed = mai_commands::build_mai_today_embed_for_now(&ctx.data().db, &display_name).await?;
    ctx.send(CreateReply::default().embed(embed)).await?;
    Ok(())
}

/// Show today's (or a specified day's) play detail as a DM (day boundary: 04:00 JST)
#[poise::command(slash_command, rename = "mai-today-detail")]
pub(crate) async fn mai_today_detail(
    ctx: Context<'_>,
    #[description = "Date in YYYY/MM/DD (default: today JST, day boundary 04:00)"] date: Option<
        String,
    >,
) -> Result<(), Error> {
    use time::{Duration as TimeDuration, Month, OffsetDateTime, UtcOffset};

    ctx.defer().await?;

    if let Err(e) = refresh_from_network_if_needed(ctx.data()).await {
        warn!("mai-today-detail: refresh failed; continuing with DB: {e:#}");
    }

    let offset = UtcOffset::from_hms(9, 0, 0).unwrap_or(UtcOffset::UTC);
    let day_date = if let Some(date) = date.as_deref() {
        let key = date.trim().replace('-', "/");
        let parts = key.split('/').collect::<Vec<_>>();
        if parts.len() != 3 {
            return Err(eyre::eyre!("date must be YYYY/MM/DD"));
        }
        let year = parts[0].parse::<i32>().wrap_err("parse year")?;
        let month = parts[1].parse::<u8>().wrap_err("parse month")?;
        let day = parts[2].parse::<u8>().wrap_err("parse day")?;
        let month = Month::try_from(month).wrap_err("parse month")?;
        time::Date::from_calendar_date(year, month, day).wrap_err("parse date")?
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
        "{:04}/{:02}/{:02}",
        day_date.year(),
        u8::from(day_date.month()),
        day_date.day()
    );
    let end_key = format!(
        "{:04}/{:02}/{:02}",
        end_date.year(),
        u8::from(end_date.month()),
        end_date.day()
    );

    let start = format!("{day_key} 04:00");
    let end = format!("{end_key} 04:00");

    let display_name = display_user_name(&ctx).await;
    let embed = mai_commands::build_mai_today_detail_embed_for_day(
        &ctx.data().db,
        ctx.data().song_data.as_deref(),
        &display_name,
        &day_key,
        &start,
        &end,
    )
    .await?;

    send_embed_dm(ctx.data(), embed).await?;

    ctx.send(CreateReply::default().ephemeral(true).embed(
        embed_base("Sent").description(format!("Sent a DM with play details for `{day_key}`.")),
    ))
    .await?;

    Ok(())
}
