use eyre::WrapErr;
use poise::CreateReply;
use poise::serenity_prelude as serenity;
use std::collections::BTreeMap;
use time::{Duration as TimeDuration, OffsetDateTime, UtcOffset};
use tracing::warn;

use crate::BotData;
use crate::chart_links::{linked_chart_label, linked_short_difficulty};
use crate::client::{
    ApiError, RecordCollectorClient, SongInfoClient, SongMetadata, SongMetadataSearchRequest,
    normalize_record_collector_url,
};
use crate::db;
use crate::embeds::{
    RecentRecordView, build_mai_recent_embeds, build_mai_today_embed, embed_base,
    embed_maintenance, format_level_with_internal,
};
use crate::emoji::{format_fc, format_rank, format_sync};

type Context<'a> = poise::Context<'a, BotData, Box<dyn std::error::Error + Send + Sync>>;
type Error = Box<dyn std::error::Error + Send + Sync>;

/// Show basic setup steps for maistats
#[poise::command(slash_command, rename = "how-to-use")]
pub(crate) async fn how_to_use(ctx: Context<'_>) -> Result<(), Error> {
    ctx.send(
        CreateReply::default().ephemeral(true).embed(
            embed_base("How to use maistats").description(
                "maistats helps you collect and manage your personal maimai records over time.\n\n\
                Open `https://maistats.muhwan.dev` to see how to set up your own record collector.\n\
                Once your collector is ready, connect it to this bot with `/register <url>`.\n\n\
                After registering, you can use commands like `/mai-score`, `/mai-recent`, `/mai-song-info`, and `/mai-today` with your own data.",
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

    ctx.send(CreateReply::default().ephemeral(true).embed(
        embed_base("Registration saved").description(format!(
            "**Player**: {}\n**Record collector**: {}",
            player_profile.user_name, normalized_url
        )),
    ))
    .await?;

    Ok(())
}

/// Get song records by song title or key
#[poise::command(slash_command, rename = "mai-score")]
pub(crate) async fn mai_score(
    ctx: Context<'_>,
    #[description = "Song title or alias to search for"] search: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let Some(record_collector_client) = registered_record_collector_client(ctx).await? else {
        return Ok(());
    };

    let requested_title = search.trim();
    if requested_title.is_empty() {
        ctx.send(
            CreateReply::default()
                .ephemeral(true)
                .embed(embed_base("No records found").description("Please provide a title.")),
        )
        .await?;
        return Ok(());
    }

    let response = ctx
        .data()
        .song_info_client
        .search_song_metadata(&SongMetadataSearchRequest {
            title: Some(requested_title.to_string()),
            genre: None,
            artist: None,
            chart_type: None,
            diff_category: None,
            limits: Some(100),
        })
        .await
        .wrap_err("search song metadata")?;

    let mut unique_songs = BTreeMap::new();
    for song in response.items {
        unique_songs
            .entry((song.title, song.genre, song.artist))
            .or_insert(());
    }

    if unique_songs.is_empty() {
        ctx.send(
            CreateReply::default()
                .ephemeral(true)
                .embed(embed_base("No records found").description("No scores for this title.")),
        )
        .await?;
        return Ok(());
    }

    if unique_songs.len() > 1 {
        let description = duplicate_song_candidates_description(&unique_songs);
        ctx.send(
            CreateReply::default()
                .ephemeral(true)
                .embed(embed_base("여러 곡이 검색됐어요").description(description)),
        )
        .await?;
        return Ok(());
    }

    let (resolved_title, resolved_genre, resolved_artist) =
        unique_songs.into_keys().next().expect("checked non-empty");

    let detailed_scores = match record_collector_client
        .get_song_detail_scores(&resolved_title, &resolved_genre, &resolved_artist)
        .await
    {
        Ok(v) => v,
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
                        return Ok(());
                    }
                    "NOT_FOUND" => {
                        ctx.send(CreateReply::default().ephemeral(true).embed(
                            embed_base("No records found").description("No scores for this title."),
                        ))
                        .await?;
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

    let embed_title = detailed_scores
        .first()
        .map(|score| score.title.as_str())
        .unwrap_or(requested_title);
    let mut embed = embed_base(embed_title)
        .field("Genre", &resolved_genre, true)
        .field("Artist", &resolved_artist, false);
    let mut has_rows = false;
    let mut first_image_name = None::<String>;

    for score in &detailed_scores {
        has_rows = true;
        let metadata = fetch_song_metadata(
            &ctx.data().song_info_client,
            &score.title,
            &resolved_genre,
            &resolved_artist,
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

        let field_name =
            linked_chart_label(&score.title, score.chart_type, score.diff_category, &level);

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
    #[description = "Song title or alias to search for"] title: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let response = ctx
        .data()
        .song_info_client
        .search_song_metadata(&SongMetadataSearchRequest {
            title: Some(title.trim().to_string()),
            genre: None,
            artist: None,
            chart_type: None,
            diff_category: None,
            limits: Some(100),
        })
        .await
        .wrap_err("search song metadata")?;

    let mut songs = BTreeMap::<(String, String, String), Vec<SongMetadata>>::new();
    for item in response.items {
        songs
            .entry((item.title.clone(), item.genre.clone(), item.artist.clone()))
            .or_default()
            .push(item);
    }

    if songs.is_empty() {
        ctx.send(
            CreateReply::default()
                .ephemeral(true)
                .embed(embed_base("No song found").description("No matching title or alias.")),
        )
        .await?;
        return Ok(());
    }

    if songs.len() > 2 {
        let candidates = songs
            .keys()
            .take(8)
            .map(|(title, genre, artist)| format!("`{title}` / `{genre}` / `{artist}`"))
            .collect::<Vec<_>>()
            .join("\n");
        ctx.send(CreateReply::default().ephemeral(true).embed(
            embed_base("여러 곡이 검색됐어요").description(format!(
                "정확히 일치하는 제목의 곡이 여러 개 있습니다.\n후보:\n{}",
                candidates
            )),
        ))
        .await?;
        return Ok(());
    }

    let mut embeds = Vec::new();
    let mut image_names = Vec::new();
    for ((song_title, song_genre, song_artist), mut sheets) in songs {
        sheets.sort_by_key(|sheet| (sheet.chart_type.as_u8(), sheet.diff_category.as_u8()));
        let (embed, image_name) =
            build_song_info_embed(&song_title, &song_genre, &song_artist, &sheets);
        if let Some(image_name) = image_name {
            image_names.push(image_name.to_string());
        }
        embeds.push(embed);
    }

    let mut attachments = Vec::new();
    for image_name in image_names {
        match ctx.data().song_info_client.get_cover(&image_name).await {
            Ok(bytes) => {
                attachments.push(serenity::CreateAttachment::bytes(bytes, image_name.clone()));
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

fn build_song_info_embed(
    song_title: &str,
    song_genre: &str,
    song_artist: &str,
    sheets: &[SongMetadata],
) -> (serenity::CreateEmbed, Option<String>) {
    let region_unreleased_line = build_region_unreleased_line(sheets);

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
                .find(|sheet| sheet.chart_type == chart_type && sheet.diff_category == difficulty)
            else {
                continue;
            };

            let short = linked_short_difficulty(song_title, chart_type, difficulty);

            let mut part = format!(
                "{short} {}",
                sheet.level.clone().unwrap_or_else(|| "N/A".to_string())
            );
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
    blocks.push(format!("Genre: {}\nArtist: {}", song_genre, song_artist));

    if let Some(levels) = format_levels(models::ChartType::Std) {
        blocks.push(format!("Level (STD)\n{levels}"));
    }
    if let Some(levels) = format_levels(models::ChartType::Dx) {
        blocks.push(format!("Level (DX)\n{levels}"));
    }

    let mut embed = embed_base(song_title);
    if !blocks.is_empty() {
        embed = embed.description(blocks.join("\n\n"));
    }

    let image_name = sheets.iter().find_map(|sheet| sheet.image_name.clone());
    if let Some(image_name) = image_name.as_deref() {
        embed = embed.thumbnail(format!("attachment://{image_name}"));
    }

    (embed, image_name)
}

/// Get most recent credit records
#[poise::command(slash_command, rename = "mai-recent")]
pub(crate) async fn mai_recent(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;

    let Some(record_collector_client) = registered_record_collector_client(ctx).await? else {
        return Ok(());
    };

    let display_name = load_player_display_name(&record_collector_client).await;
    let play_records = record_collector_client
        .get_recent(50)
        .await
        .wrap_err("fetch recent plays")?;

    if play_records.is_empty() {
        ctx.send(CreateReply::default().embed(embed_base("No recent records found")))
            .await?;
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
    let mut cover_image_names = std::collections::BTreeSet::new();
    for record in recent {
        let metadata = match record.diff_category {
            Some(diff_category) => match record.genre.as_deref().zip(record.artist.as_deref()) {
                Some((genre, artist)) => {
                    fetch_song_metadata(
                        &ctx.data().song_info_client,
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
            rank: record.score_rank,
            fc: record.fc,
            sync: record.sync,
        });
    }

    let embeds = build_mai_recent_embeds(&display_name, &records, None, &ctx.data().status_emojis);
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

    let Some(record_collector_client) = registered_record_collector_client(ctx).await? else {
        return Ok(());
    };

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
    Ok(())
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
) -> Result<Option<RecordCollectorClient>, Error> {
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

    Ok(Some(client))
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
    song_info_client: &SongInfoClient,
    title: &str,
    genre: &str,
    artist: &str,
    chart_type: models::ChartType,
    diff_category: models::DifficultyCategory,
) -> Option<SongMetadata> {
    let response = song_info_client
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

fn duplicate_song_candidates_description(
    candidates: &BTreeMap<(String, String, String), ()>,
) -> String {
    let lines = candidates
        .keys()
        .take(8)
        .map(|(title, genre, artist)| format!("`{title}` / `{genre}` / `{artist}`"))
        .collect::<Vec<_>>();

    format!(
        "검색어와 정확히 일치하는 곡이 여러 개 있습니다.\n후보:\n{}",
        lines.join("\n")
    )
}

fn build_region_unreleased_line(sheets: &[SongMetadata]) -> Option<String> {
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
    use super::latest_credit_len;

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
}
