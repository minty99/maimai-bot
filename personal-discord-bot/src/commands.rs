use eyre::{Result, WrapErr};
use poise::serenity_prelude as serenity;
use poise::CreateReply;
use time::{Duration as TimeDuration, OffsetDateTime, UtcOffset};

use crate::embeds::{
    build_mai_recent_embeds, build_mai_today_detail_embed, build_mai_today_embed, embed_base,
    embed_maintenance, format_level_with_internal, RecentRecordView,
};
use crate::rating_image::{render_rating_image, RatingImageEntry};
use crate::BotData;

type Context<'a> = poise::Context<'a, BotData, Box<dyn std::error::Error + Send + Sync>>;
type Error = Box<dyn std::error::Error + Send + Sync>;

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

    let matched_title = scores
        .iter()
        .find(|score| score.title.trim() == search.trim())
        .map(|score| score.title.clone());
    let Some(matched_title) = matched_title else {
        ctx.send(
            CreateReply::default()
                .ephemeral(true)
                .embed(embed_base("No records found").description("No titles to match.")),
        )
        .await?;
        return Ok(());
    };

    let detailed_scores = match ctx
        .data()
        .record_collector_client
        .get_song_detail_scores(&matched_title)
        .await
    {
        Ok(v) => v,
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("MAINTENANCE") || msg.contains("maintenance") {
                ctx.send(
                    CreateReply::default()
                        .ephemeral(true)
                        .embed(embed_maintenance()),
                )
                .await?;
                return Ok(());
            }
            return Err(e.wrap_err("fetch song detail scores").into());
        }
    };

    if detailed_scores.is_empty() {
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

    for score in &detailed_scores {
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
        let last_played = score
            .last_played_at
            .as_deref()
            .map(|v| format!("Last: {v}"));
        let play_count = score.play_count.map(|v| format!("Plays: {v}"));
        let detail_suffix = [last_played, play_count]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" • ");

        if first_image_name.is_none() {
            first_image_name = metadata.and_then(|m| m.image_name);
        }

        let field_name = format!("[{}] {} {}", score.chart_type, score.diff_category, level);

        let field_value = if detail_suffix.is_empty() {
            format!("{achievement_percent:.4}% • {rank} • {fc} • {sync}")
        } else {
            format!("{achievement_percent:.4}% • {rank} • {fc} • {sync}\n{detail_suffix}")
        };

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

/// Get full song info from song-info server
#[poise::command(slash_command, rename = "mai-song-info")]
pub(crate) async fn mai_song_info(
    ctx: Context<'_>,
    #[description = "Song title to search for"] title: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let Some(song_info) = ctx
        .data()
        .song_info_client
        .get_song_info_by_title(&title)
        .await
        .wrap_err("fetch song info")?
    else {
        ctx.send(
            CreateReply::default()
                .ephemeral(true)
                .embed(embed_base("No song found").description("No exact title match.")),
        )
        .await?;
        return Ok(());
    };

    let mut sheets = song_info.sheets.clone();
    sheets.sort_by_key(|sheet| (sheet.chart_type.as_u8(), sheet.difficulty.as_u8()));

    let std_version = sheets
        .iter()
        .find(|sheet| sheet.chart_type == models::ChartType::Std)
        .and_then(|sheet| sheet.version.as_deref());
    let dx_version = sheets
        .iter()
        .find(|sheet| sheet.chart_type == models::ChartType::Dx)
        .and_then(|sheet| sheet.version.as_deref());

    let format_levels = |chart_type: models::ChartType| -> Option<String> {
        let ordered_difficulties = [
            models::DifficultyCategory::Basic,
            models::DifficultyCategory::Advanced,
            models::DifficultyCategory::Expert,
            models::DifficultyCategory::Master,
            models::DifficultyCategory::ReMaster,
        ];

        let mut parts = Vec::new();
        for difficulty in ordered_difficulties {
            let Some(sheet) = sheets
                .iter()
                .find(|sheet| sheet.chart_type == chart_type && sheet.difficulty == difficulty)
            else {
                continue;
            };

            let short = match difficulty {
                models::DifficultyCategory::Basic => "B",
                models::DifficultyCategory::Advanced => "A",
                models::DifficultyCategory::Expert => "E",
                models::DifficultyCategory::Master => "M",
                models::DifficultyCategory::ReMaster => "R",
            };

            let mut part = format!("{short} {}", sheet.level);
            if let Some(internal) = sheet.internal_level {
                part.push_str(&format!(" ({internal:.1})"));
            }
            parts.push(part);
        }

        if parts.is_empty() {
            None
        } else {
            Some(parts.join(" / "))
        }
    };

    let mut blocks = Vec::new();
    let mut version_lines = Vec::new();
    if let Some(version) = std_version {
        version_lines.push(format!("Version (STD): {version}"));
    }
    if let Some(version) = dx_version {
        version_lines.push(format!("Version (DX): {version}"));
    }
    if !version_lines.is_empty() {
        blocks.push(version_lines.join("\n"));
    }

    if let Some(levels) = format_levels(models::ChartType::Std) {
        blocks.push(format!("Level (STD)\n{levels}"));
    }
    if let Some(levels) = format_levels(models::ChartType::Dx) {
        blocks.push(format!("Level (DX)\n{levels}"));
    }

    let mut base = embed_base(&song_info.title);
    if !blocks.is_empty() {
        base = base.description(blocks.join("\n\n"));
    }

    if let Some(image_name) = song_info.image_name.as_deref() {
        base = base.thumbnail(format!("attachment://{image_name}"));
    }
    let embeds = vec![base];

    let mut attachments = Vec::new();
    if let Some(image_name) = song_info.image_name.as_deref() {
        match ctx.data().song_info_client.get_cover(image_name).await {
            Ok(bytes) => {
                attachments.push(serenity::CreateAttachment::bytes(
                    bytes,
                    image_name.to_string(),
                ));
            }
            Err(e) => tracing::warn!("failed to fetch cover image {image_name}: {e:?}"),
        }
    }

    ctx.send(CreateReply {
        embeds,
        attachments,
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
    let mut cover_image_names = std::collections::BTreeSet::new();
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
        let image_name = metadata.as_ref().and_then(|m| m.image_name.clone());
        if let Some(name) = image_name.as_ref() {
            cover_image_names.insert(name.clone());
        }
        records.push(RecentRecordView {
            track: record.track.map(|t| t as i64),
            played_at: record.played_at,
            title: record.title,
            chart_type: record.chart_type,
            diff_category: record.diff_category,
            image_name,
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
    let mut attachments = Vec::new();
    for image_name in cover_image_names {
        match ctx.data().song_info_client.get_cover(&image_name).await {
            Ok(bytes) => {
                attachments.push(serenity::CreateAttachment::bytes(bytes, image_name));
            }
            Err(e) => tracing::warn!("failed to fetch cover image {image_name}: {e:?}"),
        }
    }

    ctx.send(CreateReply {
        embeds,
        attachments,
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

    let Some(targets) = fetch_rating_targets_or_maintenance(&ctx).await? else {
        return Ok(());
    };

    let embeds = build_mai_rating_embeds(&ctx.data().song_info_client, targets).await;
    ctx.send(CreateReply {
        embeds,
        ..Default::default()
    })
    .await?;

    Ok(())
}

/// Generate a single image containing all 50 rating target songs
#[poise::command(slash_command, rename = "mai-rating-img")]
pub(crate) async fn mai_rating_img(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;

    let Some(targets) = fetch_rating_targets_or_maintenance(&ctx).await? else {
        return Ok(());
    };

    let new_rows = build_rating_rows(&ctx.data().song_info_client, &targets.current_targets).await;
    let old_rows = build_rating_rows(&ctx.data().song_info_client, &targets.legacy_targets).await;
    let new_entries = new_rows
        .iter()
        .map(RatingImageEntry::from)
        .collect::<Vec<_>>();
    let old_entries = old_rows
        .iter()
        .map(RatingImageEntry::from)
        .collect::<Vec<_>>();

    let image_bytes =
        match render_rating_image(&ctx.data().song_info_client, &new_entries, &old_entries).await {
            Ok(bytes) => bytes,
            Err(e) => {
                tracing::error!("failed to render mai-rating image: {e:#}");
                ctx.send(
                    CreateReply::default().ephemeral(true).embed(
                        embed_base("Failed to render rating image").description(e.to_string()),
                    ),
                )
                .await?;
                return Ok(());
            }
        };

    ctx.send(CreateReply {
        attachments: vec![serenity::CreateAttachment::bytes(
            image_bytes,
            "mai_rating.png",
        )],
        ..Default::default()
    })
    .await?;

    Ok(())
}

async fn fetch_rating_targets_or_maintenance(
    ctx: &Context<'_>,
) -> Result<Option<models::ParsedRatingTargets>, Error> {
    match ctx
        .data()
        .record_collector_client
        .get_rating_targets()
        .await
    {
        Ok(v) => Ok(Some(v)),
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("MAINTENANCE") || msg.contains("maintenance") {
                ctx.send(
                    CreateReply::default()
                        .ephemeral(true)
                        .embed(embed_maintenance()),
                )
                .await?;
                return Ok(None);
            }
            Err(e.wrap_err("fetch rating targets").into())
        }
    }
}

#[derive(Debug, Clone)]
struct RatedRow {
    title: String,
    chart_type: models::ChartType,
    diff_category: models::DifficultyCategory,
    level: String,
    internal_level: Option<f32>,
    achievement_percent: Option<f64>,
    rank: Option<models::ScoreRank>,
    rating_points: Option<u32>,
    image_name: Option<String>,
}

impl From<&RatedRow> for RatingImageEntry {
    fn from(value: &RatedRow) -> Self {
        Self {
            title: value.title.clone(),
            chart_type: value.chart_type,
            diff_category: value.diff_category,
            level: value.level.clone(),
            internal_level: value.internal_level,
            achievement_percent: value.achievement_percent,
            rank: value.rank,
            rating_points: value.rating_points,
            image_name: value.image_name.clone(),
        }
    }
}

async fn build_mai_rating_embeds(
    song_info_client: &crate::client::SongInfoClient,
    targets: models::ParsedRatingTargets,
) -> Vec<serenity::builder::CreateEmbed> {
    let new_rows = build_rating_rows(song_info_client, &targets.current_targets).await;
    let old_rows = build_rating_rows(song_info_client, &targets.legacy_targets).await;

    let new_sum = new_rows.iter().filter_map(|r| r.rating_points).sum::<u32>();
    let old_sum = old_rows.iter().filter_map(|r| r.rating_points).sum::<u32>();
    let total = new_sum.saturating_add(old_sum);
    let missing_internal = new_rows
        .iter()
        .chain(old_rows.iter())
        .filter(|r| r.internal_level.is_none())
        .count();

    fn list_desc(rows: &[RatedRow]) -> String {
        let mut out = String::new();
        for (idx, r) in rows.iter().enumerate() {
            let rank = r.rank.map(|v| v.as_str()).unwrap_or("N/A");
            let level = format_level_with_internal(&r.level, r.internal_level);
            let achv = r
                .achievement_percent
                .map(|v| format!("{v:.4}%"))
                .unwrap_or_else(|| "N/A".to_string());
            let pt = r
                .rating_points
                .map(|v| format!("{v:>3}pt"))
                .unwrap_or_else(|| "N/A".to_string());

            out.push_str(&format!(
                "- [{}] `{}` {} [{}] {} {} — {} • {}\n",
                idx + 1,
                pt,
                r.title,
                r.chart_type,
                r.diff_category,
                level,
                achv,
                rank
            ));
        }
        out
    }

    let mut summary = embed_base("Rating (From Rating Target Music)")
        .field("Computed", total.to_string(), true)
        .field("NEW", new_sum.to_string(), true)
        .field("OLD", old_sum.to_string(), true)
        .field(
            "Count",
            format!("NEW {} / OLD {}", new_rows.len(), old_rows.len()),
            false,
        );
    if missing_internal > 0 {
        summary = summary.field(
            "Notes",
            format!("internal level missing for {missing_internal} chart(s)"),
            false,
        );
    }

    let new_embed = embed_base("Songs for Rating(New)").description(list_desc(&new_rows));
    let old_embed = embed_base("Songs for Rating(Others)").description(list_desc(&old_rows));

    vec![summary, new_embed, old_embed]
}

async fn build_rating_rows(
    song_info_client: &crate::client::SongInfoClient,
    entries: &[models::ParsedRatingTargetEntry],
) -> Vec<RatedRow> {
    let mut out = Vec::with_capacity(entries.len());
    for entry in entries {
        let metadata = fetch_song_metadata(
            song_info_client,
            &entry.title,
            entry.chart_type,
            entry.diff_category,
        )
        .await;

        let level = metadata
            .as_ref()
            .and_then(|m| m.level.clone())
            .unwrap_or_else(|| entry.level.clone());
        let internal_level = metadata
            .as_ref()
            .and_then(|m| m.internal_level)
            .or_else(|| fallback_internal_level(&level));
        let achievement_percent = entry.achievement_percent.map(|v| v as f64);
        let rating_points = match (internal_level, achievement_percent) {
            (Some(internal), Some(achv)) => Some(chart_rating_points(internal as f64, achv, false)),
            _ => None,
        };

        out.push(RatedRow {
            title: entry.title.clone(),
            chart_type: entry.chart_type,
            diff_category: entry.diff_category,
            level,
            internal_level,
            achievement_percent,
            rank: entry.rank,
            rating_points,
            image_name: metadata.and_then(|m| m.image_name),
        });
    }
    out
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

fn fallback_internal_level(level: &str) -> Option<f32> {
    let level = level.trim();
    if level.is_empty() || level == "N/A" {
        return None;
    }

    let has_plus = level.ends_with('+');
    let numeric = if has_plus {
        level.trim_end_matches('+')
    } else {
        level
    };
    let base: f32 = numeric.trim().parse().ok()?;
    Some(base + if has_plus { 0.6 } else { 0.0 })
}
