use eyre::WrapErr;
use models::SongAliases;
use poise::CreateReply;
use poise::serenity_prelude as serenity;
use time::{Duration as TimeDuration, OffsetDateTime, UtcOffset};
use tracing::warn;

use models::is_minor_or_more_outdated;

use crate::BotData;
use crate::chart_links::{linked_chart_label, linked_short_difficulty};
use crate::client::{
    ApiError, BOT_VERSION, RecordCollectorClient, RecordCollectorVersionIssue, SongCatalogSheet,
    SongCatalogSong, SongDatabaseClient, SongMetadata, normalize_record_collector_url,
};

/// Changelog entries ordered newest-first.
/// Add one entry here every time the workspace version is bumped.
const CHANGELOG: &[(&str, &str)] = &[(
    "1.1.0",
    "Commands now trigger an immediate data poll before responding, \
         so results always reflect your most recent play session.",
)];
use crate::db;
use crate::embeds::{
    RecentRecordView, build_mai_recent_embeds, build_mai_today_embed, embed_base,
    embed_maintenance, format_level_with_internal,
};
use crate::emoji::{format_fc, format_rank, format_sync};
use crate::plot;
use crate::updown;

type Context<'a> = poise::Context<'a, BotData, Box<dyn std::error::Error + Send + Sync>>;
type Error = Box<dyn std::error::Error + Send + Sync>;
const VERSION_WARNING_INTERVAL_SECONDS: i64 = 24 * 60 * 60;

#[derive(Debug, Clone)]
struct PendingRecordCollectorWarning {
    cache_key: String,
    message: String,
}

struct RegisteredRecordCollectorContext {
    client: RecordCollectorClient,
    pending_warning: Option<PendingRecordCollectorWarning>,
}

fn version_warning_cache_key(user_id: serenity::UserId, record_collector_url: &str) -> String {
    format!("{}::{record_collector_url}", user_id)
}

/// Show basic setup steps for maistats
#[poise::command(slash_command, rename = "how-to-use")]
pub(crate) async fn how_to_use(ctx: Context<'_>) -> Result<(), Error> {
    ctx.send(
        CreateReply::default().ephemeral(true).embed(
            embed_base("How to use maistats").description(
                "maistats helps you collect and manage your personal maimai records over time.\n\n\
                Open `https://maistats.muhwan.dev` to see how to set up your own record collector.\n\
                Once your collector is ready, connect it to this bot with `/register <url>`.\n\n\
                After registering, you can use commands like `/mai-score`, `/mai-recent`, `/mai-song-info`, `/mai-today`, and `/mai-updown` with your own data.",
            ),
        ),
    )
    .await?;

    Ok(())
}

/// Register your record collector server
#[poise::command(slash_command)]
pub(crate) async fn register(
    ctx: Context<'_>,
    #[description = "Record collector server base URL"] url: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let normalized_url = match normalize_record_collector_url(&url) {
        Ok(url) => url,
        Err(err) => {
            ctx.send(
                CreateReply::default()
                    .ephemeral(true)
                    .embed(embed_base("Registration failed").description(err.to_string())),
            )
            .await?;
            return Ok(());
        }
    };

    let record_collector_client = RecordCollectorClient::new(normalized_url.clone())
        .wrap_err("create record collector client")?;

    if let Err(err) = record_collector_client.health_check().await {
        send_registration_validation_error(ctx, "Registration failed", &err.to_string()).await?;
        return Ok(());
    }

    let player_profile = match record_collector_client.get_player_profile().await {
        Ok(player_profile) => player_profile,
        Err(err) => {
            if let Some(api_error) = err.downcast_ref::<ApiError>()
                && api_error.code() == "MAINTENANCE"
            {
                ctx.send(
                    CreateReply::default()
                        .ephemeral(true)
                        .embed(embed_maintenance()),
                )
                .await?;
                return Ok(());
            }

            let description = match err.downcast_ref::<ApiError>() {
                Some(api_error) if !api_error.message().is_empty() => api_error.message(),
                _ => "Record collector validation failed.",
            };
            send_registration_validation_error(ctx, "Registration failed", description).await?;
            return Ok(());
        }
    };

    db::upsert_registration(
        &ctx.data().db_pool,
        ctx.author().id,
        &normalized_url,
        OffsetDateTime::now_utc().unix_timestamp(),
    )
    .await
    .wrap_err("save registration")?;

    let (title, description) =
        registration_success_message(&player_profile.user_name, &normalized_url);

    ctx.send(
        CreateReply::default()
            .ephemeral(true)
            .embed(embed_base(title).description(description)),
    )
    .await?;

    let pending_warning = prepare_record_collector_update_warning(
        ctx.author().id,
        &normalized_url,
        &record_collector_client,
    )
    .await;
    send_pending_record_collector_update_warning(ctx, pending_warning).await?;

    Ok(())
}

/// Get song records by song title or key
#[poise::command(slash_command, rename = "mai-score")]
pub(crate) async fn mai_score(
    ctx: Context<'_>,
    #[description = "Song title or alias to search for"] search: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let Some(collector_context) = registered_record_collector_client(ctx).await? else {
        return Ok(());
    };
    let record_collector_client = collector_context.client;
    let pending_warning = collector_context.pending_warning;

    let requested_title = search.trim();
    if requested_title.is_empty() {
        ctx.send(
            CreateReply::default()
                .ephemeral(true)
                .embed(embed_base("No records found").description("Please provide a title.")),
        )
        .await?;
        send_pending_record_collector_update_warning(ctx, pending_warning).await?;
        return Ok(());
    }

    let matched_songs = search_song_catalog(&ctx.data().song_database_client, requested_title)
        .await
        .wrap_err("search song catalog")?;

    if matched_songs.is_empty() {
        ctx.send(
            CreateReply::default()
                .ephemeral(true)
                .embed(embed_base("No song found").description("No matching title or alias.")),
        )
        .await?;
        send_pending_record_collector_update_warning(ctx, pending_warning).await?;
        return Ok(());
    }

    if matched_songs.len() > 1 {
        ctx.send(
            CreateReply::default()
                .ephemeral(true)
                .embed(build_duplicate_song_candidates_embed(&matched_songs)),
        )
        .await?;
        send_pending_record_collector_update_warning(ctx, pending_warning).await?;
        return Ok(());
    }

    let resolved_song = matched_songs.into_iter().next().expect("checked non-empty");

    let detailed_scores = match record_collector_client
        .get_song_detail_scores(
            &resolved_song.title,
            &resolved_song.genre,
            &resolved_song.artist,
        )
        .await
    {
        Ok(mut v) => {
            v.sort_by_key(|s| (s.chart_type.as_u8(), s.diff_category));
            v
        }
        Err(e) => {
            if let Some(api_error) = e.downcast_ref::<ApiError>() {
                match api_error.code() {
                    "MAINTENANCE" => {
                        ctx.send(
                            CreateReply::default()
                                .ephemeral(true)
                                .embed(embed_maintenance()),
                        )
                        .await?;
                        send_pending_record_collector_update_warning(ctx, pending_warning).await?;
                        return Ok(());
                    }
                    "NOT_FOUND" => {
                        send_no_records_found_reply(ctx, &resolved_song).await?;
                        send_pending_record_collector_update_warning(ctx, pending_warning).await?;
                        return Ok(());
                    }
                    _ => {}
                }
            }

            let msg = e.to_string();
            if msg.contains("maintenance") {
                ctx.send(
                    CreateReply::default()
                        .ephemeral(true)
                        .embed(embed_maintenance()),
                )
                .await?;
                send_pending_record_collector_update_warning(ctx, pending_warning).await?;
                return Ok(());
            }
            return Err(e.wrap_err("fetch song detail scores").into());
        }
    };

    if detailed_scores.is_empty() {
        send_no_records_found_reply(ctx, &resolved_song).await?;
        send_pending_record_collector_update_warning(ctx, pending_warning).await?;
        return Ok(());
    }

    let embed_title = detailed_scores
        .first()
        .map(|score| score.title.as_str())
        .unwrap_or(requested_title);
    let mut has_rows = false;
    let mut first_image_name = None::<String>;
    let mut desc_blocks: Vec<String> = Vec::new();

    for score in &detailed_scores {
        has_rows = true;
        let metadata = fetch_song_metadata(
            &ctx.data().song_database_client,
            &score.title,
            &resolved_song.genre,
            &resolved_song.artist,
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
        let rank = format_rank(&ctx.data().status_emojis, score.rank, "N/A");
        let fc = format_fc(&ctx.data().status_emojis, score.fc, "-");
        let sync = format_sync(&ctx.data().status_emojis, score.sync, "-");
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

        let chart_line =
            linked_chart_label(&score.title, score.chart_type, score.diff_category, &level);
        let score_line = format!("{achievement_percent:.4}% • {rank} • {fc} • {sync}");

        let block = if detail_suffix.is_empty() {
            format!("**{chart_line}**\n{score_line}")
        } else {
            format!("**{chart_line}**\n{score_line}\n{detail_suffix}")
        };
        desc_blocks.push(block);
    }

    let mut embed = embed_base(embed_title).description(desc_blocks.join("\n\n"));
    if let Some(ref image_name) = first_image_name {
        embed = embed.thumbnail(ctx.data().song_database_client.cover_url(image_name));
    }

    ctx.send(CreateReply {
        embeds: vec![embed],
        ephemeral: Some(!has_rows),
        ..Default::default()
    })
    .await?;
    send_pending_record_collector_update_warning(ctx, pending_warning).await?;

    Ok(())
}

/// Get full song info from the shared song database
#[poise::command(slash_command, rename = "mai-song-info")]
pub(crate) async fn mai_song_info(
    ctx: Context<'_>,
    #[description = "Song title or alias to search for"] title: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let requested_title = title.trim();
    if requested_title.is_empty() {
        ctx.send(
            CreateReply::default()
                .ephemeral(true)
                .embed(embed_base("No song found").description("Please provide a title.")),
        )
        .await?;
        return Ok(());
    }

    let matched_songs = search_song_catalog(&ctx.data().song_database_client, requested_title)
        .await
        .wrap_err("search song catalog")?;

    if matched_songs.is_empty() {
        ctx.send(
            CreateReply::default()
                .ephemeral(true)
                .embed(embed_base("No song found").description("No matching title or alias.")),
        )
        .await?;
        return Ok(());
    }

    if matched_songs.len() > 1 {
        ctx.send(
            CreateReply::default()
                .ephemeral(true)
                .embed(build_duplicate_song_candidates_embed(&matched_songs)),
        )
        .await?;
        return Ok(());
    }

    let mut embeds = Vec::new();
    for mut song in matched_songs {
        song.sheets
            .sort_by_key(|sheet| (sheet.chart_type.as_u8(), sheet.diff_category.as_u8()));
        let mut embed = build_song_info_embed(&song);
        if let Some(image_name) = song.image_name.as_deref() {
            embed = embed.thumbnail(ctx.data().song_database_client.cover_url(image_name));
        }
        embeds.push(embed);
    }

    ctx.send(CreateReply {
        embeds,
        ..Default::default()
    })
    .await?;

    Ok(())
}

fn build_song_info_embed(song: &SongCatalogSong) -> serenity::CreateEmbed {
    let region_unreleased_line = build_region_unreleased_line(&song.sheets);

    let std_version = song
        .sheets
        .iter()
        .find(|sheet| sheet.chart_type == models::ChartType::Std)
        .and_then(|sheet| sheet.version.as_deref());
    let dx_version = song
        .sheets
        .iter()
        .find(|sheet| sheet.chart_type == models::ChartType::Dx)
        .and_then(|sheet| sheet.version.as_deref());

    let format_levels =
        |chart_type: models::ChartType| -> Option<String> {
            let ordered_difficulties = [
                models::DifficultyCategory::Basic,
                models::DifficultyCategory::Advanced,
                models::DifficultyCategory::Expert,
                models::DifficultyCategory::Master,
                models::DifficultyCategory::ReMaster,
            ];

            let mut parts = Vec::new();
            for difficulty in ordered_difficulties {
                let Some(sheet) = song.sheets.iter().find(|sheet| {
                    sheet.chart_type == chart_type && sheet.diff_category == difficulty
                }) else {
                    continue;
                };

                let short = linked_short_difficulty(&song.title, chart_type, difficulty);

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
    if let Some(region_line) = region_unreleased_line {
        blocks.push(region_line);
    }

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
    blocks.push(format!("Genre: {}\nArtist: {}", song.genre, song.artist));

    if let Some(levels) = format_levels(models::ChartType::Std) {
        blocks.push(format!("Level (STD)\n{levels}"));
    }
    if let Some(levels) = format_levels(models::ChartType::Dx) {
        blocks.push(format!("Level (DX)\n{levels}"));
    }

    let mut embed = embed_base(&song.title);
    if !blocks.is_empty() {
        embed = embed.description(blocks.join("\n\n"));
    }

    embed
}

/// Get most recent credit records
#[poise::command(slash_command, rename = "mai-recent")]
pub(crate) async fn mai_recent(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;

    let Some(collector_context) = registered_record_collector_client(ctx).await? else {
        return Ok(());
    };
    let record_collector_client = collector_context.client;
    let pending_warning = collector_context.pending_warning;

    let display_name = load_player_display_name(&record_collector_client).await;
    let play_records = record_collector_client
        .get_recent(50)
        .await
        .wrap_err("fetch recent plays")?;

    if play_records.is_empty() {
        ctx.send(CreateReply::default().embed(embed_base("No recent records found")))
            .await?;
        send_pending_record_collector_update_warning(ctx, pending_warning).await?;
        return Ok(());
    }

    let tracks: Vec<Option<i64>> = play_records
        .iter()
        .map(|r| r.track.map(|t| t as i64))
        .collect();
    let take = latest_credit_len(&tracks);

    let mut recent = play_records.into_iter().take(take).collect::<Vec<_>>();
    recent.reverse();

    let mut records = Vec::with_capacity(recent.len());
    for record in recent {
        let metadata = match record.diff_category {
            Some(diff_category) => match record.genre.as_deref().zip(record.artist.as_deref()) {
                Some((genre, artist)) => {
                    fetch_song_metadata(
                        &ctx.data().song_database_client,
                        &record.title,
                        genre,
                        artist,
                        record.chart_type,
                        diff_category,
                    )
                    .await
                }
                None => None,
            },
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
            rank: record.score_rank,
            fc: record.fc,
            sync: record.sync,
        });
    }

    let embeds = build_mai_recent_embeds(
        &display_name,
        &records,
        None,
        &ctx.data().status_emojis,
        &ctx.data().song_database_client,
    );

    ctx.send(CreateReply {
        embeds,
        ..Default::default()
    })
    .await?;
    send_pending_record_collector_update_warning(ctx, pending_warning).await?;

    Ok(())
}

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

    let Some(collector_context) = registered_record_collector_client(ctx).await? else {
        return Ok(());
    };
    let record_collector_client = collector_context.client;
    let pending_warning = collector_context.pending_warning;

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

    let plays = record_collector_client.get_today(&today_str).await?;

    let tracks = plays.len() as i64;
    let credits = plays
        .iter()
        .filter_map(|p| p.credit_id)
        .collect::<std::collections::HashSet<_>>()
        .len() as i64;
    let new_records = plays
        .iter()
        .filter(|p| p.achievement_new_record.unwrap_or(0) != 0)
        .count() as i64;

    let start = format!("{} 04:00", today_str);
    let end = format!(
        "{:04}-{:02}-{:02} 04:00",
        end_date.year(),
        u8::from(end_date.month()),
        end_date.day()
    );

    let display_name = load_player_display_name(&record_collector_client).await;
    let embed = build_mai_today_embed(&display_name, &start, &end, credits, tracks, new_records);

    ctx.send(CreateReply::default().embed(embed)).await?;
    send_pending_record_collector_update_warning(ctx, pending_warning).await?;
    Ok(())
}

/// Start a mai-updown random session in a thread
#[poise::command(slash_command, rename = "mai-updown", guild_only)]
pub(crate) async fn mai_updown(
    ctx: Context<'_>,
    #[description = "Starting internal level (for example 13.0)"] internal_level: f64,
) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;

    let Some(collector_context) = registered_record_collector_client(ctx).await? else {
        return Ok(());
    };
    let record_collector_client = collector_context.client;
    let pending_warning = collector_context.pending_warning;

    let reply = match updown::parse_level_tenths(internal_level) {
        Ok(start_level_tenths) => {
            match updown::start_session(ctx, record_collector_client, start_level_tenths).await {
                Ok(()) => CreateReply::default()
                    .ephemeral(true)
                    .content("mai-updown session started."),
                Err(err) => build_mai_updown_start_error_reply(&err),
            }
        }
        Err(err) => CreateReply::default()
            .ephemeral(true)
            .embed(embed_base("Invalid internal level").description(err.to_string())),
    };

    ctx.send(reply).await?;

    send_pending_record_collector_update_warning(ctx, pending_warning).await?;

    Ok(())
}

/// Scatter plot of best achievements for charts in an internal level range (played in last 3 months)
#[poise::command(slash_command, rename = "mai-plot")]
pub(crate) async fn mai_plot(
    ctx: Context<'_>,
    #[description = "Internal level range start (e.g. 13.0)"] from: f64,
    #[description = "Internal level range end — same as start for a single level (e.g. 13.9)"]
    to: f64,
) -> Result<(), Error> {
    ctx.defer().await?;

    let Some(collector_context) = registered_record_collector_client(ctx).await? else {
        return Ok(());
    };
    let record_collector_client = collector_context.client;
    let pending_warning = collector_context.pending_warning;

    /// Validate that a raw f64 Discord input is a multiple of 0.1 in [1.0, 15.0].
    /// Returns the integer tenths value (e.g. 13.0 → 130) or an error description.
    fn parse_level_tenths(v: f64) -> Result<i32, &'static str> {
        let tenths = (v * 10.0).round() as i32;
        if (v * 10.0 - tenths as f64).abs() > 0.01 {
            return Err("Level must have at most one decimal place (e.g. 13.0, 14.7).");
        }
        if !(10..=150).contains(&tenths) {
            return Err("Level must be between 1.0 and 15.0.");
        }
        Ok(tenths)
    }

    let from_tenths = match parse_level_tenths(from) {
        Ok(t) => t,
        Err(msg) => {
            ctx.send(
                CreateReply::default()
                    .ephemeral(true)
                    .embed(embed_base("Invalid `from` level").description(msg)),
            )
            .await?;
            send_pending_record_collector_update_warning(ctx, pending_warning).await?;
            return Ok(());
        }
    };
    let to_tenths = match parse_level_tenths(to) {
        Ok(t) => t,
        Err(msg) => {
            ctx.send(
                CreateReply::default()
                    .ephemeral(true)
                    .embed(embed_base("Invalid `to` level").description(msg)),
            )
            .await?;
            send_pending_record_collector_update_warning(ctx, pending_warning).await?;
            return Ok(());
        }
    };
    if from_tenths > to_tenths {
        ctx.send(
            CreateReply::default().ephemeral(true).embed(
                embed_base("Invalid range").description("`from` level must be ≤ `to` level."),
            ),
        )
        .await?;
        send_pending_record_collector_update_warning(ctx, pending_warning).await?;
        return Ok(());
    }

    let level_range_str = if from_tenths == to_tenths {
        format!("{:.1}", from_tenths as f32 / 10.0)
    } else {
        format!(
            "{:.1}–{:.1}",
            from_tenths as f32 / 10.0,
            to_tenths as f32 / 10.0
        )
    };

    // JST threshold string for last_played_at comparison ("YYYY/MM/DD HH:MM")
    let jst = UtcOffset::from_hms(9, 0, 0).unwrap_or(UtcOffset::UTC);
    let since_jst = (OffsetDateTime::now_utc() - TimeDuration::days(90)).to_offset(jst);
    let since_jst_str = format!(
        "{:04}/{:02}/{:02} {:02}:{:02}",
        since_jst.year(),
        u8::from(since_jst.month()),
        since_jst.day(),
        since_jst.hour(),
        since_jst.minute(),
    );

    let scores = record_collector_client
        .get_all_rated_scores()
        .await
        .wrap_err("fetch rated scores")?;

    let catalog = ctx
        .data()
        .song_database_client
        .list_song_catalog()
        .await
        .wrap_err("fetch song catalog")?;

    // Build lookup: (title, genre, artist, chart_type, diff_category) → internal_level_tenths
    use models::{ChartType, DifficultyCategory};
    use std::collections::HashMap;

    let mut level_map: HashMap<(String, String, String, ChartType, DifficultyCategory), i32> =
        HashMap::new();
    for song in &catalog {
        for sheet in &song.sheets {
            if let Some(il) = sheet.internal_level {
                let il_tenths = (il * 10.0).round() as i32;
                level_map.insert(
                    (
                        song.title.clone(),
                        song.genre.clone(),
                        song.artist.clone(),
                        sheet.chart_type,
                        sheet.diff_category,
                    ),
                    il_tenths,
                );
            }
        }
    }

    const MIN_ACHIEVEMENT: i64 = 900_000; // 90.0000%

    // Each point: (achievement_percent, level_tenths)
    let mut points: Vec<(f64, i32)> = Vec::new();

    for score in &scores {
        let Some(achievement) = score.achievement_x10000 else {
            continue;
        };
        if achievement < MIN_ACHIEVEMENT {
            continue;
        }

        let Some(ref last_played) = score.last_played_at else {
            continue;
        };
        if last_played.as_str() < since_jst_str.as_str() {
            continue;
        }

        let key = (
            score.title.clone(),
            score.genre.clone(),
            score.artist.clone(),
            score.chart_type,
            score.diff_category,
        );
        let Some(&il_tenths) = level_map.get(&key) else {
            continue;
        };
        if il_tenths < from_tenths || il_tenths > to_tenths {
            continue;
        }

        points.push((achievement as f64 / 10000.0, il_tenths));
    }

    if points.is_empty() {
        ctx.send(CreateReply::default().embed(
            embed_base(format!("No records for level {}", level_range_str).as_str()).description(
                "No songs in this level range with ≥90% were played in the last 3 months.",
            ),
        ))
        .await?;
        send_pending_record_collector_update_warning(ctx, pending_warning).await?;
        return Ok(());
    }

    let total = points.len();
    let min_achievement = points.iter().map(|&(x, _)| x).fold(f64::INFINITY, f64::min);
    let x_min = min_achievement.min(100.5);

    let png = plot::generate_scatter_plot(&points, x_min)
        .await
        .wrap_err("generate scatter plot")?;

    use poise::serenity_prelude::builder::CreateAttachment;

    ctx.send(
        CreateReply::default()
            .content(format!(
                "Level **{}** — **{}** song{} with ≥90% played in the last 3 months",
                level_range_str,
                total,
                if total == 1 { "" } else { "s" },
            ))
            .attachment(CreateAttachment::bytes(png, "plot.png")),
    )
    .await?;

    send_pending_record_collector_update_warning(ctx, pending_warning).await?;
    Ok(())
}

fn build_mai_updown_start_error_reply(err: &Error) -> CreateReply {
    if let Some(api_error) = err.downcast_ref::<ApiError>()
        && api_error.code() == "MAINTENANCE"
    {
        CreateReply::default()
            .ephemeral(true)
            .embed(embed_maintenance())
    } else {
        CreateReply::default()
            .ephemeral(true)
            .embed(embed_base("Unable to start mai-updown").description(err.to_string()))
    }
}

async fn send_registration_validation_error(
    ctx: Context<'_>,
    title: &str,
    description: &str,
) -> Result<(), Error> {
    ctx.send(
        CreateReply::default()
            .ephemeral(true)
            .embed(embed_base(title).description(description)),
    )
    .await?;
    Ok(())
}

async fn registered_record_collector_client(
    ctx: Context<'_>,
) -> Result<Option<RegisteredRecordCollectorContext>, Error> {
    let Some(registration) = db::get_registration(&ctx.data().db_pool, ctx.author().id)
        .await
        .wrap_err("load user registration")?
    else {
        ctx.send(CreateReply::default().ephemeral(true).embed(
            embed_base("Registration required").description(
                "Please run `/how-to-use` for setup instructions, then connect your record collector with `/register <url>`.",
            ),
        ))
        .await?;
        return Ok(None);
    };

    let client = match RecordCollectorClient::new(registration.record_collector_server_url.clone())
    {
        Ok(client) => client,
        Err(err) => {
            ctx.send(
                CreateReply::default().ephemeral(true).embed(
                    embed_base("Invalid registration").description(format!(
                        "Your stored record collector URL is invalid. Please re-run `/register <url>`.\n\n{}",
                        err
                    )),
                ),
            )
            .await?;
            return Ok(None);
        }
    };

    client.trigger_poll().await;

    let pending_warning = prepare_record_collector_update_warning(
        registration.discord_user_id,
        &registration.record_collector_server_url,
        &client,
    )
    .await;

    Ok(Some(RegisteredRecordCollectorContext {
        client,
        pending_warning,
    }))
}

fn registration_success_message(player_name: &str, normalized_url: &str) -> (&'static str, String) {
    (
        "Registration saved",
        [
            format!("**Player**: {player_name}"),
            format!("**Record collector**: {normalized_url}"),
        ]
        .join("\n"),
    )
}

async fn prepare_record_collector_update_warning(
    discord_user_id: serenity::UserId,
    record_collector_server_url: &str,
    client: &RecordCollectorClient,
) -> Option<PendingRecordCollectorWarning> {
    let version_status = client.get_version_status().await;
    let issue = version_status.issue()?;

    let cache_key = version_warning_cache_key(discord_user_id, record_collector_server_url);
    Some(PendingRecordCollectorWarning {
        cache_key,
        message: build_record_collector_update_message(
            client.base_url(),
            issue,
            version_status.collector_version(),
        ),
    })
}

async fn send_pending_record_collector_update_warning(
    ctx: Context<'_>,
    pending_warning: Option<PendingRecordCollectorWarning>,
) -> Result<(), Error> {
    let Some(pending_warning) = pending_warning else {
        return Ok(());
    };

    let now_unix = OffsetDateTime::now_utc().unix_timestamp();
    if let Ok(mut cache) = ctx.data().version_warning_cache.lock() {
        cache.retain(|_, last_sent_at| now_unix - *last_sent_at < VERSION_WARNING_INTERVAL_SECONDS);
        let warned_recently = cache.contains_key(&pending_warning.cache_key);
        if warned_recently {
            return Ok(());
        }
    }

    ctx.send(CreateReply::default().ephemeral(true).embed(
        embed_base("Record collector update required").description(pending_warning.message),
    ))
    .await?;

    if let Ok(mut cache) = ctx.data().version_warning_cache.lock() {
        cache.insert(pending_warning.cache_key, now_unix);
    }

    Ok(())
}

fn changelog_since(collector_version: &str) -> String {
    let entries: Vec<_> = CHANGELOG
        .iter()
        .filter(|(entry_version, _)| {
            // Keep entries that are newer than the collector's version.
            // is_minor_or_more_outdated(entry, collector) == true means collector < entry.
            is_minor_or_more_outdated(entry_version, collector_version).unwrap_or(false)
        })
        .collect();

    if entries.is_empty() {
        return String::new();
    }

    let lines = entries
        .iter()
        .map(|(v, desc)| format!("**{v}** — {desc}"))
        .collect::<Vec<_>>()
        .join("\n");

    format!("\n\n**What's new:**\n{lines}")
}

fn build_record_collector_update_message(
    record_collector_url: &str,
    issue: RecordCollectorVersionIssue,
    collector_version: Option<&str>,
) -> String {
    match issue {
        RecordCollectorVersionIssue::VersionMismatch => {
            let collector_ver = collector_version.unwrap_or("unknown");
            let changelog = changelog_since(collector_ver);
            format!(
                "Your record collector is outdated.\n\
                 Bot version: `{BOT_VERSION}`\n\
                 Collector version: `{collector_ver}`\n\
                 Please update the server before relying on the bot.\n\
                 ```\ndocker compose up -d --pull always\n```{changelog}"
            )
        }
        RecordCollectorVersionIssue::InvalidResponse => format!(
            "Your record collector returned an invalid semantic version from `/api/version`.\n\
             Bot version: `{BOT_VERSION}`\n\
             Collector version: `{}`\n\
             Please update the server.\n\
             ```\ndocker compose up -d --pull always\n```",
            collector_version.unwrap_or("unknown")
        ),
        RecordCollectorVersionIssue::Unreachable => format!(
            "The bot could not verify `{}/api/version`.\n\
            Collectors that do not expose this endpoint are treated as outdated.\n\
             Expected version: `{BOT_VERSION}`\n\
             Please update the server.\n\
             ```\ndocker compose up -d --pull always\n```",
            record_collector_url.trim_end_matches('/')
        ),
    }
}

async fn load_player_display_name(record_collector_client: &RecordCollectorClient) -> String {
    match record_collector_client.get_player_profile().await {
        Ok(player_profile) => player_profile.user_name,
        Err(err) => {
            warn!("failed to load player display name: {err:#}");
            "Player".to_string()
        }
    }
}

async fn fetch_song_metadata(
    song_database_client: &SongDatabaseClient,
    title: &str,
    genre: &str,
    artist: &str,
    chart_type: models::ChartType,
    diff_category: models::DifficultyCategory,
) -> Option<SongMetadata> {
    let response = song_database_client
        .find_song_metadata(title, genre, artist, chart_type, diff_category)
        .await;

    match response {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(
                "failed to fetch song metadata for {title} [{chart_type} {diff_category}]: {e:#}"
            );
            None
        }
    }
}

async fn send_no_records_found_reply(
    ctx: Context<'_>,
    song: &SongCatalogSong,
) -> Result<(), Error> {
    let mut embed = embed_base(&song.title).description("No records found.");

    if let Some(image_name) = song.image_name.as_deref() {
        embed = embed.thumbnail(ctx.data().song_database_client.cover_url(image_name));
    }

    ctx.send(CreateReply {
        embeds: vec![embed],
        ephemeral: Some(true),
        ..Default::default()
    })
    .await?;

    Ok(())
}

async fn search_song_catalog(
    song_database_client: &SongDatabaseClient,
    query: &str,
) -> eyre::Result<Vec<SongCatalogSong>> {
    let songs = song_database_client.list_song_catalog().await?;
    Ok(find_song_candidates(songs, query))
}

fn build_duplicate_song_candidates_embed(candidates: &[SongCatalogSong]) -> serenity::CreateEmbed {
    let shown = candidates.len().min(8);
    let mut description =
        "Search matched multiple songs. Please try a more specific title or alias.".to_string();
    if candidates.len() > shown {
        description.push_str(&format!(
            "\nShowing first {shown} of {} candidates.",
            candidates.len()
        ));
    }

    let mut embed = embed_base("Multiple songs found").description(description);
    for candidate in candidates.iter().take(shown) {
        embed = embed.field(
            &candidate.title,
            format_song_candidate_details(&candidate.genre, &candidate.artist, &candidate.aliases),
            false,
        );
    }

    embed
}

fn format_song_candidate_details(genre: &str, artist: &str, aliases: &SongAliases) -> String {
    let mut line = format!("Genre: {genre}\nArtist: {artist}");
    if let Some(alias_summary) = format_song_alias_summary(aliases) {
        line.push_str(&format!("\nAliases: {alias_summary}"));
    }
    line
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum SongSearchMatchKind {
    Exact,
    CaseInsensitiveExact,
    WhitespaceInsensitiveExact,
    Contains,
}

fn find_song_candidates(mut songs: Vec<SongCatalogSong>, query: &str) -> Vec<SongCatalogSong> {
    let mut exact_matches = Vec::new();
    let mut case_insensitive_matches = Vec::new();
    let mut whitespace_insensitive_matches = Vec::new();
    let mut contains_matches = Vec::new();

    for song in songs.drain(..) {
        match song_match_kind(&song, query) {
            Some(SongSearchMatchKind::Exact) => exact_matches.push(song),
            Some(SongSearchMatchKind::CaseInsensitiveExact) => case_insensitive_matches.push(song),
            Some(SongSearchMatchKind::WhitespaceInsensitiveExact) => {
                whitespace_insensitive_matches.push(song);
            }
            Some(SongSearchMatchKind::Contains) => contains_matches.push(song),
            None => {}
        }
    }

    for matches in [
        &mut exact_matches,
        &mut case_insensitive_matches,
        &mut whitespace_insensitive_matches,
        &mut contains_matches,
    ] {
        if matches.is_empty() {
            continue;
        }

        matches
            .sort_by(|a, b| (&a.title, &a.genre, &a.artist).cmp(&(&b.title, &b.genre, &b.artist)));
        return std::mem::take(matches);
    }

    Vec::new()
}

fn song_match_kind(song: &SongCatalogSong, query: &str) -> Option<SongSearchMatchKind> {
    let trimmed_query = query.trim();
    if trimmed_query.is_empty() {
        return None;
    }

    let search_values = song_search_values(song);
    if search_values
        .iter()
        .any(|value| value.trim() == trimmed_query)
    {
        return Some(SongSearchMatchKind::Exact);
    }

    let normalized_query = normalize_search_value(trimmed_query);
    if search_values
        .iter()
        .any(|value| normalize_search_value(value) == normalized_query)
    {
        return Some(SongSearchMatchKind::CaseInsensitiveExact);
    }

    let collapsed_query = collapse_search_value(trimmed_query);
    if search_values
        .iter()
        .any(|value| collapse_search_value(value) == collapsed_query)
    {
        return Some(SongSearchMatchKind::WhitespaceInsensitiveExact);
    }

    if normalized_query.len() < 2 && collapsed_query.len() < 2 {
        return None;
    }

    if search_values.iter().any(|value| {
        let normalized_value = normalize_search_value(value);
        let collapsed_value = collapse_search_value(value);
        normalized_value.contains(&normalized_query) || collapsed_value.contains(&collapsed_query)
    }) {
        return Some(SongSearchMatchKind::Contains);
    }

    None
}

fn song_search_values(song: &SongCatalogSong) -> Vec<&str> {
    std::iter::once(song.title.as_str())
        .chain(song.aliases.en.iter().map(String::as_str))
        .chain(song.aliases.ko.iter().map(String::as_str))
        .collect()
}

fn normalize_search_value(value: &str) -> String {
    value.trim().to_lowercase()
}

fn collapse_search_value(value: &str) -> String {
    value
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .flat_map(|ch| ch.to_lowercase())
        .collect()
}

fn format_song_alias_summary(aliases: &SongAliases) -> Option<String> {
    let mut values = Vec::new();
    for alias in aliases.en.iter().chain(aliases.ko.iter()) {
        let alias = alias.trim();
        if alias.is_empty() || values.iter().any(|existing| existing == alias) {
            continue;
        }
        values.push(alias.to_string());
    }

    if values.is_empty() {
        return None;
    }

    const MAX_ALIASES: usize = 6;
    let total = values.len();
    values.truncate(MAX_ALIASES);

    let mut summary = values.join(", ");
    if total > MAX_ALIASES {
        summary.push_str(", ...");
    }

    Some(summary)
}

fn build_region_unreleased_line(sheets: &[SongCatalogSheet]) -> Option<String> {
    let has_jp = sheets.iter().any(|sheet| sheet.region.jp);
    let has_intl = sheets.iter().any(|sheet| sheet.region.intl);

    match (has_jp, has_intl) {
        (true, false) => Some("**INTL**: Unreleased".to_string()),
        (false, true) => Some("**JP**: Unreleased".to_string()),
        _ => None,
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

#[cfg(test)]
mod tests {
    use super::{
        find_song_candidates, format_song_alias_summary, format_song_candidate_details,
        latest_credit_len,
    };
    use crate::client::SongCatalogSong;
    use models::SongAliases;

    #[test]
    fn latest_credit_len_uses_first_track_one_boundary() {
        assert_eq!(latest_credit_len(&[Some(4), Some(3), Some(2), Some(1)]), 4);
        assert_eq!(latest_credit_len(&[Some(2), Some(1), Some(4)]), 2);
    }

    #[test]
    fn latest_credit_len_falls_back_to_four_when_track_one_is_missing() {
        assert_eq!(latest_credit_len(&[Some(4), Some(3), Some(2), Some(5)]), 4);
        assert_eq!(latest_credit_len(&[Some(4), Some(3)]), 2);
    }

    #[test]
    fn format_song_alias_summary_deduplicates_and_limits_values() {
        let aliases = SongAliases {
            en: vec![
                "aaa".to_string(),
                "bbb".to_string(),
                "ccc".to_string(),
                "ddd".to_string(),
            ],
            ko: vec![
                "bbb".to_string(),
                "eee".to_string(),
                "fff".to_string(),
                "ggg".to_string(),
            ],
        };

        assert_eq!(
            format_song_alias_summary(&aliases).as_deref(),
            Some("aaa, bbb, ccc, ddd, eee, fff, ...")
        );
    }

    #[test]
    fn format_song_candidate_details_is_english_and_includes_aliases() {
        let details = format_song_candidate_details(
            "POPS & ANIME",
            "Composer",
            &SongAliases {
                en: vec!["alias-a".to_string()],
                ko: vec!["별칭".to_string()],
            },
        );

        assert!(details.contains("Genre: POPS & ANIME"));
        assert!(details.contains("Artist: Composer"));
        assert!(details.contains("Aliases: alias-a, 별칭"));
    }

    #[test]
    fn find_song_candidates_prefers_whitespace_insensitive_exact_over_contains() {
        let matches = find_song_candidates(
            vec![
                test_song("Night of Nights", "alias"),
                test_song("Nightwalker", "night"),
            ],
            "nightofnights",
        );

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].title, "Night of Nights");
    }

    #[test]
    fn find_song_candidates_returns_multiple_contains_matches() {
        let matches = find_song_candidates(
            vec![
                test_song("Alpha Song", "first"),
                test_song("Beta Song", "second"),
                test_song("Gamma", "third"),
            ],
            "song",
        );

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].title, "Alpha Song");
        assert_eq!(matches[1].title, "Beta Song");
    }

    #[test]
    fn find_song_candidates_matches_alias_case_insensitively() {
        let matches = find_song_candidates(vec![test_song("Real Title", "My Alias")], "my alias");

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].title, "Real Title");
    }

    fn test_song(title: &str, alias: &str) -> SongCatalogSong {
        SongCatalogSong {
            title: title.to_string(),
            genre: "POPS & ANIME".to_string(),
            artist: "Composer".to_string(),
            image_name: None,
            aliases: SongAliases {
                en: vec![alias.to_string()],
                ko: Vec::new(),
            },
            sheets: Vec::new(),
        }
    }
}
