use eyre::{Result, WrapErr};
use poise::serenity_prelude as serenity;
use poise::{CreateReply, FrameworkOptions};
use reqwest::Url;
use serenity::builder::{CreateEmbed, CreateMessage};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{debug, error, info, warn};

use crate::config::AppConfig;
use crate::db;
use crate::db::{SqlitePool, format_chart_type, format_percent_f64};
use crate::discord::mai_commands;
use crate::http::MaimaiClient;
use crate::http::is_maintenance_window_now;
use crate::maimai::models::{ParsedPlayRecord, ParsedPlayerData};
use crate::maimai::parse::player_data::parse_player_data_html;
use crate::maimai::parse::recent::parse_recent_html;
use crate::maimai::parse::score_list::parse_scores_html;
use crate::maimai::rating::{chart_rating_points, is_ap_like};
use crate::song_data::SongDataIndex;

type Context<'a> = poise::Context<'a, BotData, Error>;
type Error = eyre::Report;

const STATE_KEY_TOTAL_PLAY_COUNT: &str = "player.total_play_count";
const STATE_KEY_RATING: &str = "player.rating";

const EMBED_COLOR: u32 = 0x51BCF3;

#[derive(Debug, Clone)]
pub struct BotData {
    pub db: SqlitePool,
    pub maimai_client: Arc<MaimaiClient>,
    pub config: AppConfig,
    pub discord_user_id: serenity::UserId,
    pub discord_http: Arc<serenity::Http>,
    pub maimai_user_name: Arc<RwLock<String>>,
    pub song_data: Option<Arc<SongDataIndex>>,
}

pub async fn run_bot(config: AppConfig, db_path: std::path::PathBuf) -> Result<()> {
    info!("Initializing database at {:?}", db_path);
    let pool = db::connect(&db_path)
        .await
        .wrap_err("connect to database")?;
    db::migrate(&pool)
        .await
        .wrap_err("run database migrations")?;
    info!("Database initialized successfully");

    let maimai_client = Arc::new(MaimaiClient::new(&config).wrap_err("create HTTP client")?);

    let discord_bot_token = config
        .discord_bot_token
        .clone()
        .ok_or_else(|| eyre::eyre!("missing env var: DISCORD_BOT_TOKEN"))?;
    let discord_user_id_str = config
        .discord_user_id
        .clone()
        .ok_or_else(|| eyre::eyre!("missing env var: DISCORD_USER_ID"))?;

    let discord_http = Arc::new(serenity::Http::new(&discord_bot_token));

    let discord_user_id = serenity::UserId::new(
        discord_user_id_str
            .parse::<u64>()
            .wrap_err("parse DISCORD_USER_ID")?,
    );

    let song_data = match SongDataIndex::load_from_default_locations(&config) {
        Ok(v) => v.map(Arc::new),
        Err(e) => {
            warn!("failed to load song_data.json (non-fatal): {e:?}");
            None
        }
    };

    let bot_data = BotData {
        db: pool,
        maimai_client,
        config: config.clone(),
        discord_user_id,
        discord_http,
        maimai_user_name: Arc::new(RwLock::new(String::new())),
        song_data,
    };

    let framework = poise::Framework::builder()
        .options(FrameworkOptions {
            prefix_options: Default::default(),
            commands: vec![mai_score(), mai_recent(), mai_today(), mai_rating()],
            on_error: |error| {
                Box::pin(async move {
                    match error {
                        poise::FrameworkError::Command { error, ctx, .. } => {
                            error!(
                                "Command '{}' failed: {:?}",
                                ctx.command().qualified_name,
                                error
                            );
                            let _ = ctx
                                .send(
                                    CreateReply::default()
                                        .content("Error executing command")
                                        .ephemeral(true),
                                )
                                .await;
                        }
                        poise::FrameworkError::ArgumentParse { error, ctx, .. } => {
                            error!("Argument parse error: {:?}", error);
                            let _ = ctx
                                .send(
                                    CreateReply::default()
                                        .content("Invalid arguments")
                                        .ephemeral(true),
                                )
                                .await;
                        }
                        _ => {
                            error!("Framework error: {:?}", error);
                        }
                    }
                })
            },
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                info!("Bot started as {}", ctx.cache.current_user().name);

                if is_maintenance_window_now() {
                    info!(
                        "Skipping startup crawl due to maintenance window (04:00-07:00 local time)"
                    );
                    start_background_tasks(bot_data.clone(), ctx.cache.clone());

                    poise::builtins::register_globally(ctx, &framework.options().commands)
                        .await
                        .wrap_err("register commands globally")?;
                    return Ok(bot_data);
                }

                let player_data = fetch_player_data(&bot_data)
                    .await
                    .wrap_err("fetch player data")?;
                *bot_data.maimai_user_name.write().await = player_data.user_name.clone();

                if should_sync_scores(&bot_data.db, &player_data)
                    .await
                    .wrap_err("check whether scores sync is needed")?
                {
                    initial_scores_sync(&bot_data)
                        .await
                        .wrap_err("startup scores sync")?;
                    initial_recent_sync(&bot_data, player_data.total_play_count)
                        .await
                        .wrap_err("startup recent sync")?;
                    persist_play_counts(&bot_data.db, &player_data)
                        .await
                        .wrap_err("persist play counts")?;
                } else {
                    info!(
                        "Skipping startup scores sync (play count unchanged: total={})",
                        player_data.total_play_count
                    );
                }

                start_background_tasks(bot_data.clone(), ctx.cache.clone());

                poise::builtins::register_globally(ctx, &framework.options().commands)
                    .await
                    .wrap_err("register commands globally")?;

                send_startup_dm(&bot_data, &player_data)
                    .await
                    .wrap_err("send startup DM")?;

                Ok(bot_data)
            })
        })
        .build();

    let intents = serenity::GatewayIntents::GUILDS;

    let mut client = serenity::Client::builder(&discord_bot_token, intents)
        .framework(framework)
        .await
        .wrap_err("create Discord client")?;

    info!("Starting Discord bot...");
    client.start().await.wrap_err("client error")?;

    Ok(())
}

async fn display_user_name(ctx: &poise::Context<'_, BotData, Error>) -> String {
    let name = ctx.data().maimai_user_name.read().await.clone();
    if name.trim().is_empty() {
        ctx.author().name.clone()
    } else {
        name
    }
}

fn embed_base(title: &str) -> CreateEmbed {
    let mut e = CreateEmbed::new();
    e = e.title(title).color(EMBED_COLOR);
    e
}

fn format_delta(current: u32, previous: Option<u32>) -> String {
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

fn embed_startup(player: &ParsedPlayerData) -> CreateEmbed {
    let play_count = format!(
        "{} ({})",
        player.total_play_count, player.current_version_play_count
    );
    embed_base("maimai-bot started")
        .field("User", &player.user_name, true)
        .field("Rating", player.rating.to_string(), true)
        .field("Play count", play_count, true)
}

fn embed_player_update(
    player: &ParsedPlayerData,
    prev_total: Option<u32>,
    prev_rating: Option<u32>,
    credit_entries: &[ParsedPlayRecord],
) -> CreateEmbed {
    let mut e = embed_base("New plays detected")
        .field("Rating", format_delta(player.rating, prev_rating), true)
        .field(
            "Play count",
            format_delta(player.total_play_count, prev_total),
            true,
        );

    if !credit_entries.is_empty() {
        let records = credit_entries
            .iter()
            .map(|r| CreditRecordView {
                track: r.track.map(i64::from),
                played_at: r.played_at.clone(),
                title: r.title.clone(),
                chart_type: format_chart_type(r.chart_type).to_string(),
                diff_category: r.diff_category.map(|d| d.as_str().to_string()),
                level: r.level.clone(),
                achievement_percent: r.achievement_percent.map(|p| p as f64),
                rank: r.score_rank.map(|rk| rk.as_str().to_string()),
            })
            .collect::<Vec<_>>();

        e = e.description(format_credit_description(&records));
    }

    e
}

#[derive(Debug, Clone)]
struct CreditRecordView {
    track: Option<i64>,
    played_at: Option<String>,
    title: String,
    chart_type: String,
    diff_category: Option<String>,
    level: Option<String>,
    achievement_percent: Option<f64>,
    rank: Option<String>,
}

#[derive(Debug, Clone)]
struct RecentRecordView {
    track: Option<i64>,
    played_at: Option<String>,
    title: String,
    chart_type: String,
    diff_category: Option<String>,
    level: Option<String>,
    internal_level: Option<f32>,
    rating_points: Option<u32>,
    achievement_percent: Option<f64>,
    rank: Option<String>,
}

#[derive(Debug, Clone)]
struct ScoreRowView {
    chart_type: String,
    diff_category: String,
    level: String,
    internal_level: Option<f32>,
    rating_points: Option<u32>,
    achievement_percent: Option<f64>,
    rank: Option<String>,
}

fn format_level_with_internal(level: &str, internal_level: Option<f32>) -> String {
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

fn build_mai_score_embed(display_name: &str, title: &str, entries: &[ScoreRowView]) -> CreateEmbed {
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

fn build_mai_recent_embeds(display_name: &str, records: &[RecentRecordView]) -> Vec<CreateEmbed> {
    records
        .iter()
        .map(|record| {
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
            let mut embed = embed_base(&track).description(format!(
                "**{}** [{}] {diff} {level} — {achv} • {rank}{rating}",
                record.title, record.chart_type
            ));
            if let Some(played_at) = record.played_at.as_deref() {
                embed = embed.field("Played at", played_at, false);
            }
            let _ = display_name;
            embed
        })
        .collect::<Vec<_>>()
}

fn build_mai_today_embed(
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

fn format_credit_description(records: &[CreditRecordView]) -> String {
    let played_at = records
        .iter()
        .find(|r| r.track == Some(1))
        .and_then(|r| r.played_at.as_deref())
        .unwrap_or("N/A");

    let mut desc = String::new();
    desc.push_str(&format!("`{played_at}`\n\n"));

    for r in records {
        let track = format_track_label(r.track);
        let achv = format_percent_f64(r.achievement_percent);
        let rank = r
            .rank
            .as_deref()
            .map(normalize_playlog_rank)
            .unwrap_or("N/A");
        let diff = r.diff_category.as_deref().unwrap_or("Unknown");
        let level = r.level.as_deref().unwrap_or("N/A");

        desc.push_str(&format!("**{track}**\n"));
        desc.push_str(&format!(
            "**{}** [{}] {diff} {level} — {achv} • {rank}\n\n",
            r.title, r.chart_type
        ));
    }

    desc
}

async fn initial_scores_sync(bot_data: &BotData) -> Result<()> {
    info!("Running startup scores sync (diff 0..4)...");

    let mut client = MaimaiClient::new(&bot_data.config).wrap_err("create HTTP client")?;
    client
        .ensure_logged_in()
        .await
        .wrap_err("ensure logged in")?;

    let count = rebuild_scores_with_client(&bot_data.db, &client)
        .await
        .wrap_err("rebuild scores")?;

    info!("Startup scores sync completed: entries={count}");
    Ok(())
}

async fn rebuild_scores_with_client(pool: &SqlitePool, client: &MaimaiClient) -> Result<usize> {
    db::clear_scores(pool).await.wrap_err("clear scores")?;

    let scraped_at = unix_timestamp();
    let mut all = Vec::new();

    for diff in 0u8..=4 {
        let url = scores_url(diff).wrap_err("build scores url")?;
        let bytes = client.get_bytes(&url).await.wrap_err("fetch scores url")?;
        let html = String::from_utf8(bytes).wrap_err("scores response is not utf-8")?;
        let mut entries = parse_scores_html(&html, diff).wrap_err("parse scores html")?;
        all.append(&mut entries);
    }

    let count = all.len();
    db::upsert_scores(pool, scraped_at, &all)
        .await
        .wrap_err("upsert scores")?;

    Ok(count)
}

async fn initial_recent_sync(bot_data: &BotData, total_play_count: u32) -> Result<()> {
    info!("Running startup recent sync...");

    let mut client = MaimaiClient::new(&bot_data.config).wrap_err("create HTTP client")?;
    client
        .ensure_logged_in()
        .await
        .wrap_err("ensure logged in")?;

    let entries = fetch_recent_entries_logged_in(&client)
        .await
        .wrap_err("fetch recent entries")?;

    let entries = annotate_recent_entries_with_play_count(entries, total_play_count);
    let scraped_at = unix_timestamp();
    let count_total = entries.len();
    let count_with_idx = entries
        .iter()
        .filter(|e| e.played_at_unixtime.is_some())
        .count();

    db::upsert_playlogs(&bot_data.db, scraped_at, &entries)
        .await
        .wrap_err("upsert playlogs")?;

    info!(
        "Startup recent sync completed: entries_total={count_total} entries_with_idx={count_with_idx}"
    );
    Ok(())
}

fn start_background_tasks(bot_data: BotData, _cache: Arc<serenity::Cache>) {
    tokio::spawn(async move {
        let mut timer = interval(Duration::from_secs(600));
        timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        info!("Background task started: periodic playerData poll (every 10 minutes)");

        loop {
            timer.tick().await;

            info!("Running periodic playerData poll...");

            if let Err(e) = periodic_player_poll(&bot_data).await {
                error!("Periodic poll failed: {}", e);
            }
        }
    });
}

async fn periodic_player_poll(bot_data: &BotData) -> Result<()> {
    if is_maintenance_window_now() {
        info!("Skipping periodic poll due to maintenance window (04:00-07:00 local time)");
        return Ok(());
    }

    let mut client = MaimaiClient::new(&bot_data.config).wrap_err("create HTTP client")?;
    client
        .ensure_logged_in()
        .await
        .wrap_err("ensure logged in")?;

    let player_data = fetch_player_data_logged_in(&client)
        .await
        .wrap_err("fetch player data")?;
    *bot_data.maimai_user_name.write().await = player_data.user_name.clone();

    let stored_total = db::get_app_state_u32(&bot_data.db, STATE_KEY_TOTAL_PLAY_COUNT).await;
    let stored_rating = db::get_app_state_u32(&bot_data.db, STATE_KEY_RATING).await;

    let stored_total = match stored_total {
        Ok(v) => v,
        Err(e) => {
            debug!("Failed to read stored total play count; treating as missing: {e:#}");
            None
        }
    };
    let stored_rating = match stored_rating {
        Ok(v) => v,
        Err(e) => {
            debug!("Failed to read stored rating; treating as missing: {e:#}");
            None
        }
    };

    if let Some(stored_total) = stored_total
        && stored_total == player_data.total_play_count
    {
        return Ok(());
    }

    let entries = fetch_recent_entries_logged_in(&client)
        .await
        .wrap_err("fetch recent")?;

    let mut entries =
        annotate_recent_entries_with_play_count(entries, player_data.total_play_count);

    if stored_total.is_some() {
        annotate_first_play_flags(&bot_data.db, &mut entries)
            .await
            .wrap_err("classify first plays")?;
    }
    let scraped_at = unix_timestamp();

    db::upsert_playlogs(&bot_data.db, scraped_at, &entries)
        .await
        .wrap_err("upsert playlogs")?;

    rebuild_scores_with_client(&bot_data.db, &client)
        .await
        .wrap_err("rebuild scores")?;
    persist_player_snapshot(&bot_data.db, &player_data)
        .await
        .wrap_err("persist player snapshot")?;

    let credit_entries = latest_credit_entries(&entries);

    if stored_total.is_some() {
        send_player_update_dm(
            bot_data,
            stored_total,
            stored_rating,
            &player_data,
            &credit_entries,
        )
        .await
        .wrap_err("send player update DM")?;
    } else {
        debug!("No stored total play count; seeded DB without sending DM");
    }

    Ok(())
}

async fn fetch_player_data(bot_data: &BotData) -> Result<ParsedPlayerData> {
    let mut client = MaimaiClient::new(&bot_data.config).wrap_err("create HTTP client")?;
    client
        .ensure_logged_in()
        .await
        .wrap_err("ensure logged in")?;

    fetch_player_data_logged_in(&client).await
}

async fn fetch_player_data_logged_in(client: &MaimaiClient) -> Result<ParsedPlayerData> {
    let url = Url::parse("https://maimaidx-eng.com/maimai-mobile/playerData/")
        .wrap_err("parse playerData url")?;
    let bytes = client
        .get_bytes(&url)
        .await
        .wrap_err("fetch playerData url")?;
    let html = String::from_utf8(bytes).wrap_err("playerData response is not utf-8")?;
    parse_player_data_html(&html).wrap_err("parse playerData html")
}

async fn fetch_recent_entries_logged_in(client: &MaimaiClient) -> Result<Vec<ParsedPlayRecord>> {
    let url = Url::parse("https://maimaidx-eng.com/maimai-mobile/record/")
        .wrap_err("parse record url")?;
    let bytes = client.get_bytes(&url).await.wrap_err("fetch record url")?;
    let html = String::from_utf8(bytes).wrap_err("record response is not utf-8")?;
    parse_recent_html(&html).wrap_err("parse recent html")
}

async fn should_sync_scores(pool: &SqlitePool, player_data: &ParsedPlayerData) -> Result<bool> {
    match db::get_app_state_u32(pool, STATE_KEY_TOTAL_PLAY_COUNT).await {
        Ok(Some(v)) => Ok(v != player_data.total_play_count),
        Ok(None) => {
            debug!("No stored total play count; will rebuild DB");
            Ok(true)
        }
        Err(e) => {
            debug!("Failed to read total play count from DB; will rebuild DB: {e:#}");
            Ok(true)
        }
    }
}

async fn persist_play_counts(pool: &SqlitePool, player_data: &ParsedPlayerData) -> Result<()> {
    let now = unix_timestamp();
    db::set_app_state_u32(
        pool,
        STATE_KEY_TOTAL_PLAY_COUNT,
        player_data.total_play_count,
        now,
    )
    .await
    .wrap_err("store total play count")?;
    Ok(())
}

async fn persist_player_snapshot(pool: &SqlitePool, player_data: &ParsedPlayerData) -> Result<()> {
    let now = unix_timestamp();
    db::set_app_state_u32(
        pool,
        STATE_KEY_TOTAL_PLAY_COUNT,
        player_data.total_play_count,
        now,
    )
    .await
    .wrap_err("store total play count")?;
    db::set_app_state_u32(pool, STATE_KEY_RATING, player_data.rating, now)
        .await
        .wrap_err("store rating")?;
    Ok(())
}

async fn send_startup_dm(bot_data: &BotData, player_data: &ParsedPlayerData) -> Result<()> {
    let http = &bot_data.discord_http;
    let dm_channel = bot_data
        .discord_user_id
        .create_dm_channel(http)
        .await
        .wrap_err("create DM channel")?;

    dm_channel
        .send_message(http, CreateMessage::new().embed(embed_startup(player_data)))
        .await
        .wrap_err("send DM")?;
    Ok(())
}

/// Get song records by song title or key
#[poise::command(slash_command, rename = "mai-score")]
async fn mai_score(
    ctx: Context<'_>,
    #[description = "Song title to search for"] search: String,
) -> Result<(), Error> {
    ctx.defer().await?;

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
    let (embed, has_rows) = mai_commands::build_mai_score_embed_for_title(
        &ctx.data().db,
        ctx.data().song_data.as_deref(),
        &display_name,
        &matched_title,
    )
    .await?;

    let reply = CreateReply::default().embed(embed).ephemeral(!has_rows);
    ctx.send(reply).await?;

    Ok(())
}

fn normalize_for_match(s: &str) -> String {
    s.to_ascii_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
}

fn top_title_matches(search: &str, titles: &[String], limit: usize) -> Vec<String> {
    let search_norm = normalize_for_match(search.trim());
    let mut scored = titles
        .iter()
        .map(|t| (t, levenshtein(&search_norm, &normalize_for_match(t))))
        .collect::<Vec<_>>();
    scored.sort_by_key(|(_, d)| *d);
    scored
        .into_iter()
        .take(limit.max(1))
        .map(|(t, _)| t.clone())
        .collect()
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a = a.as_bytes();
    let b = b.as_bytes();
    if a.is_empty() {
        return b.len();
    }
    if b.is_empty() {
        return a.len();
    }

    let mut prev = (0..=b.len()).collect::<Vec<usize>>();
    let mut cur = vec![0usize; b.len() + 1];

    for (i, &ac) in a.iter().enumerate() {
        cur[0] = i + 1;
        for (j, &bc) in b.iter().enumerate() {
            let cost = usize::from(ac != bc);
            cur[j + 1] = (prev[j + 1] + 1).min(cur[j] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }

    prev[b.len()]
}

fn latest_credit_len(tracks: &[Option<i64>]) -> usize {
    match tracks.iter().position(|t| *t == Some(1)) {
        Some(idx) => idx + 1,
        None => tracks.len().min(4),
    }
}

/// Get most recent credit records
#[poise::command(slash_command, rename = "mai-recent")]
async fn mai_recent(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;

    let display_name = display_user_name(&ctx).await;
    let embeds = mai_commands::build_mai_recent_embeds_for_latest_credit(
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

/// Show rating breakdown (CiRCLE baseline)
#[poise::command(slash_command, rename = "mai-rating")]
async fn mai_rating(ctx: Context<'_>) -> Result<(), Error> {
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
async fn mai_today(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;

    let display_name = display_user_name(&ctx).await;
    let embed = mai_commands::build_mai_today_embed_for_now(&ctx.data().db, &display_name).await?;
    ctx.send(CreateReply::default().embed(embed)).await?;
    Ok(())
}

fn normalize_playlog_rank(rank: &str) -> &str {
    match rank {
        "SSSPLUS" => "SSS+",
        "SSPLUS" => "SS+",
        "SPLUS" => "S+",
        _ => rank,
    }
}

fn latest_credit_entries(entries: &[ParsedPlayRecord]) -> Vec<ParsedPlayRecord> {
    let take = latest_credit_len(
        &entries
            .iter()
            .map(|e| e.track.map(i64::from))
            .collect::<Vec<_>>(),
    );
    let mut out = entries.iter().take(take).cloned().collect::<Vec<_>>();
    out.reverse();
    out
}

fn annotate_recent_entries_with_play_count(
    mut entries: Vec<ParsedPlayRecord>,
    total_play_count: u32,
) -> Vec<ParsedPlayRecord> {
    let Some(last_track_01_idx) = entries.iter().rposition(|e| e.track == Some(1)) else {
        return Vec::new();
    };
    entries.truncate(last_track_01_idx + 1);

    let mut credit_idx: u32 = 0;
    for entry in &mut entries {
        entry.credit_play_count = Some(total_play_count.saturating_sub(credit_idx));

        if entry.track == Some(1) {
            credit_idx = credit_idx.saturating_add(1);
        }
    }

    entries
}

async fn annotate_first_play_flags(
    pool: &SqlitePool,
    entries: &mut [ParsedPlayRecord],
) -> Result<()> {
    for entry in entries {
        if !entry.achievement_new_record {
            continue;
        }
        let Some(diff_category) = entry.diff_category else {
            continue;
        };

        let existing = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT 1
            FROM scores
            WHERE title = ?1
              AND chart_type = ?2
              AND diff_category = ?3
              AND achievement_x10000 IS NOT NULL
            LIMIT 1
            "#,
        )
        .bind(&entry.title)
        .bind(format_chart_type(entry.chart_type))
        .bind(diff_category.as_str())
        .fetch_optional(pool)
        .await
        .wrap_err("check existing score")?;

        if existing.is_none() {
            entry.first_play = true;
        }
    }

    Ok(())
}

async fn send_player_update_dm(
    bot_data: &BotData,
    prev_total: Option<u32>,
    prev_rating: Option<u32>,
    current: &ParsedPlayerData,
    credit_entries: &[ParsedPlayRecord],
) -> Result<()> {
    let http = &bot_data.discord_http;
    let dm_channel = bot_data
        .discord_user_id
        .create_dm_channel(http)
        .await
        .wrap_err("create DM channel")?;

    dm_channel
        .send_message(
            http,
            CreateMessage::new().embed(embed_player_update(
                current,
                prev_total,
                prev_rating,
                credit_entries,
            )),
        )
        .await
        .wrap_err("send DM")?;
    Ok(())
}

fn unix_timestamp() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn scores_url(diff: u8) -> Result<Url> {
    if diff > 4 {
        return Err(eyre::eyre!("diff must be 0..4"));
    }
    Url::parse(&format!(
        "https://maimaidx-eng.com/maimai-mobile/record/musicGenre/search/?genre=99&diff={diff}"
    ))
    .wrap_err("parse scores url")
}

#[cfg(test)]
mod preview_tests;

#[cfg(test)]
mod tests {
    use dotenvy::dotenv;

    use super::latest_credit_len;

    #[test]
    fn latest_credit_len_stops_at_first_track_01() {
        let tracks = vec![Some(4), Some(3), Some(2), Some(1), Some(4), Some(3)];
        assert_eq!(latest_credit_len(&tracks), 4);
    }

    #[test]
    fn latest_credit_len_includes_only_one_track() {
        let tracks = vec![Some(1), Some(4), Some(3), Some(2)];
        assert_eq!(latest_credit_len(&tracks), 1);
    }

    #[test]
    fn latest_credit_len_falls_back_when_missing() {
        let tracks = vec![Some(4), Some(3), Some(2)];
        assert_eq!(latest_credit_len(&tracks), 3);
        let tracks = vec![Some(4), Some(3), Some(2), Some(4), Some(3)];
        assert_eq!(latest_credit_len(&tracks), 4);
    }

    #[tokio::test]
    #[ignore = "Sends a real DM to preview embed UI; requires DISCORD_BOT_TOKEN and DISCORD_USER_ID"]
    async fn preview_embed_player_update_dm() -> eyre::Result<()> {
        use super::embed_player_update;
        use crate::maimai::models::{
            ChartType, DifficultyCategory, ParsedPlayRecord, ParsedPlayerData, ScoreRank,
        };
        use eyre::WrapErr;
        use poise::serenity_prelude as serenity;
        use serenity::builder::CreateMessage;

        dotenv().ok();

        let token = std::env::var("DISCORD_BOT_TOKEN").ok();
        let user_id = std::env::var("DISCORD_USER_ID").ok();
        let (Some(token), Some(user_id)) = (token, user_id) else {
            return Ok(());
        };

        let http = serenity::Http::new(&token);
        let user_id =
            serenity::UserId::new(user_id.parse::<u64>().wrap_err("parse DISCORD_USER_ID")?);

        let player = ParsedPlayerData {
            user_name: "maimai-user".to_string(),
            rating: 12345,
            current_version_play_count: 67,
            total_play_count: 890,
        };

        let credit_entries = vec![
            ParsedPlayRecord {
                played_at_unixtime: None,
                track: Some(1),
                played_at: Some("2026/01/20 12:34".to_string()),
                credit_play_count: None,
                title: "Sample Song A".to_string(),
                chart_type: ChartType::Std,
                diff_category: Some(DifficultyCategory::Expert),
                level: Some("12+".to_string()),
                achievement_percent: Some(99.1234),
                achievement_new_record: false,
                first_play: false,
                score_rank: Some(ScoreRank::SPlus),
                fc: None,
                sync: None,
                dx_score: Some(1234),
                dx_score_max: Some(1500),
            },
            ParsedPlayRecord {
                played_at_unixtime: None,
                track: Some(2),
                played_at: Some("2026/01/20 12:38".to_string()),
                credit_play_count: None,
                title: "Sample Song B".to_string(),
                chart_type: ChartType::Dx,
                diff_category: Some(DifficultyCategory::Master),
                level: Some("14".to_string()),
                achievement_percent: Some(100.0000),
                achievement_new_record: false,
                first_play: false,
                score_rank: Some(ScoreRank::SssPlus),
                fc: None,
                sync: None,
                dx_score: Some(1499),
                dx_score_max: Some(1500),
            },
        ];

        let dm = user_id
            .create_dm_channel(&http)
            .await
            .wrap_err("create DM channel")?;

        let result = dm
            .send_message(
                &http,
                CreateMessage::new().embed(embed_player_update(
                    &player,
                    Some(889),
                    Some(12340),
                    &credit_entries,
                )),
            )
            .await
            .wrap_err("send DM")?;

        println!("DM sent: {}", result.id);

        Ok(())
    }
}
