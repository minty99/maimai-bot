use eyre::{Result, WrapErr};
use poise::serenity_prelude as serenity;
use poise::{CreateReply, FrameworkOptions};
use reqwest::Url;
use serenity::builder::{CreateEmbed, CreateMessage};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tracing::{debug, error, info};

use crate::config::AppConfig;
use crate::db;
use crate::db::{
    SqlitePool, format_chart_type, format_diff_category, format_percent_f64, format_track,
};
use crate::http::MaimaiClient;
use crate::maimai::models::{ParsedPlayRecord, ParsedPlayerData};
use crate::maimai::parse::player_data::parse_player_data_html;
use crate::maimai::parse::recent::parse_recent_html;
use crate::maimai::parse::score_list::parse_scores_html;

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

    let bot_data = BotData {
        db: pool,
        maimai_client,
        config: config.clone(),
        discord_user_id,
        discord_http,
    };

    let framework = poise::Framework::builder()
        .options(FrameworkOptions {
            prefix_options: Default::default(),
            commands: vec![mai_score(), mai_recent()],
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

                let player_data = fetch_player_data(&bot_data)
                    .await
                    .wrap_err("fetch player data")?;

                if should_sync_scores(&bot_data.db, &player_data)
                    .await
                    .wrap_err("check whether scores sync is needed")?
                {
                    initial_scores_sync(&bot_data)
                        .await
                        .wrap_err("startup scores sync")?;
                    initial_recent_sync(&bot_data)
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
        .field("User", &player.user_name, true)
        .field("Rating", format_delta(player.rating, prev_rating), true)
        .field(
            "Total play count",
            format_delta(player.total_play_count, prev_total),
            true,
        );

    if !credit_entries.is_empty() {
        let mut desc = String::new();
        for record in credit_entries {
            let track = record
                .track
                .map(|t| format!("T{t:02}"))
                .unwrap_or("?".to_string());
            let played_at = record.played_at.as_deref().unwrap_or("N/A");
            let diff = format_diff_category(record.diff_category);
            let level = record.level.as_deref().unwrap_or("N/A");
            let achv = format_percent_f64(record.achievement_percent.map(|p| p as f64));
            let rank = record.score_rank.as_deref().unwrap_or("N/A");
            desc.push_str(&format!(
                "`{track}` **{}** [{}] {diff} {level}\n{achv} • {rank} • {played_at}\n\n",
                record.title,
                format_chart_type(record.chart_type),
            ));
        }
        e = e.description(desc);
    }

    e
}

async fn initial_scores_sync(bot_data: &BotData) -> Result<()> {
    info!("Running startup scores sync (diff 0..4)...");

    let mut client = MaimaiClient::new(&bot_data.config).wrap_err("create HTTP client")?;
    client
        .ensure_logged_in()
        .await
        .wrap_err("ensure logged in")?;

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
    db::upsert_scores(&bot_data.db, scraped_at, &all)
        .await
        .wrap_err("upsert scores")?;

    info!("Startup scores sync completed: entries={count}");
    Ok(())
}

async fn initial_recent_sync(bot_data: &BotData) -> Result<()> {
    info!("Running startup recent sync...");

    let mut client = MaimaiClient::new(&bot_data.config).wrap_err("create HTTP client")?;
    client
        .ensure_logged_in()
        .await
        .wrap_err("ensure logged in")?;

    let entries = fetch_recent_entries_logged_in(&client)
        .await
        .wrap_err("fetch recent entries")?;
    let scraped_at = unix_timestamp();
    let count_total = entries.len();
    let count_with_idx = entries.iter().filter(|e| e.playlog_idx.is_some()).count();

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
    let mut client = MaimaiClient::new(&bot_data.config).wrap_err("create HTTP client")?;
    client
        .ensure_logged_in()
        .await
        .wrap_err("ensure logged in")?;

    let player_data = fetch_player_data_logged_in(&client)
        .await
        .wrap_err("fetch player data")?;

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
    let scraped_at = unix_timestamp();

    db::upsert_playlogs(&bot_data.db, scraped_at, &entries)
        .await
        .wrap_err("upsert playlogs")?;
    persist_player_snapshot(&bot_data.db, &player_data)
        .await
        .wrap_err("persist player snapshot")?;

    let credit_entries = most_recent_credit_entries(&entries);

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

    let rows = sqlx::query_as::<_, (String, String, String, String, Option<f64>, Option<String>)>(
        r#"
        SELECT
            sc.title,
            sc.chart_type,
            sc.diff_category,
            sc.level,
            sc.achievement_x10000 / 10000.0 as achievement_percent,
            sc.rank
        FROM scores sc
        WHERE sc.title LIKE ?
        ORDER BY
            sc.chart_type,
            CASE sc.diff_category
                WHEN 'BASIC' THEN 0
                WHEN 'ADVANCED' THEN 1
                WHEN 'EXPERT' THEN 2
                WHEN 'MASTER' THEN 3
                WHEN 'Re:MASTER' THEN 4
                ELSE 255
            END
        "#,
    )
    .bind(format!("%{}%", search))
    .fetch_all(&ctx.data().db)
    .await
    .map_err(|e| eyre::eyre!("query failed: {}", e))?;

    if rows.is_empty() {
        ctx.send(
            CreateReply::default()
                .embed(embed_base("No records found").description(format!("Query: `{search}`"))),
        )
        .await?;
        return Ok(());
    }

    let mut titles = Vec::<String>::new();
    let mut grouped = std::collections::BTreeMap::<
        String,
        Vec<(String, String, String, Option<f64>, Option<String>)>,
    >::new();

    for (title, chart_type, diff_category, level, achievement, rank) in rows {
        if !grouped.contains_key(&title) {
            titles.push(title.clone());
        }
        grouped.entry(title).or_default().push((
            chart_type,
            diff_category,
            level,
            achievement,
            rank,
        ));
    }

    let mut desc = String::new();
    for title in titles {
        let Some(mut entries) = grouped.remove(&title) else {
            continue;
        };
        entries.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));

        desc.push_str(&format!("**{}**\n", title));
        for (chart_type, diff_category, level, achievement, rank) in entries {
            let achv = format_percent_f64(achievement);
            let rank = rank.unwrap_or_else(|| "N/A".to_string());
            desc.push_str(&format!(
                "- [{}] {diff_category} {level} — {achv} • {rank}\n",
                chart_type
            ));
        }
        desc.push('\n');
    }

    ctx.send(
        CreateReply::default().embed(embed_base("Scores").description(desc).field(
            "Query",
            format!("`{search}`"),
            false,
        )),
    )
    .await?;

    Ok(())
}

fn latest_credit_count(tracks: &[Option<i64>]) -> usize {
    match tracks.iter().position(|t| *t == Some(1)) {
        Some(idx) => idx + 1,
        None => tracks.len().min(4),
    }
}

/// Get most recent credit records
#[poise::command(slash_command, rename = "mai-recent")]
async fn mai_recent(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;

    let rows = sqlx::query_as::<
        _,
        (
            String,
            String,
            Option<i64>,
            Option<String>,
            Option<f64>,
            Option<String>,
        ),
    >(
        r#"
        SELECT
            pl.title,
            pl.chart_type,
            pl.track,
            pl.played_at,
            pl.achievement_x10000 / 10000.0 as achievement_percent,
            pl.score_rank
        FROM playlogs pl
        WHERE pl.playlog_idx IS NOT NULL
        ORDER BY pl.played_at DESC
        LIMIT 50
        "#,
    )
    .fetch_all(&ctx.data().db)
    .await
    .map_err(|e| eyre::eyre!("query failed: {}", e))?;

    if rows.is_empty() {
        ctx.send(CreateReply::default().embed(embed_base("No recent records found")))
            .await?;
        return Ok(());
    }

    let take = latest_credit_count(&rows.iter().map(|row| row.2).collect::<Vec<_>>());
    let mut desc = String::new();
    for (title, chart_type, track, played_at, achievement, rank) in rows.into_iter().take(take) {
        let played_at = played_at.unwrap_or_else(|| "N/A".to_string());
        let achv = format_percent_f64(achievement);
        let rank = rank.unwrap_or_else(|| "N/A".to_string());
        desc.push_str(&format!(
            "`{}` **{}** [{}]\n{achv} • {rank} • {played_at}\n\n",
            format_track(track),
            title,
            chart_type
        ));
    }

    ctx.send(
        CreateReply::default().embed(embed_base("Recent 1 Credit").description(desc).field(
            "Plays",
            take.to_string(),
            true,
        )),
    )
    .await?;

    Ok(())
}

fn most_recent_credit_entries(entries: &[ParsedPlayRecord]) -> Vec<ParsedPlayRecord> {
    let take = latest_credit_count_u8(&entries.iter().map(|e| e.track).collect::<Vec<_>>());
    entries.iter().take(take).cloned().collect()
}

fn latest_credit_count_u8(tracks: &[Option<u8>]) -> usize {
    match tracks.iter().position(|t| *t == Some(1)) {
        Some(idx) => idx + 1,
        None => tracks.len().min(4),
    }
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
mod tests {
    use super::{latest_credit_count, latest_credit_count_u8};

    #[test]
    fn latest_credit_count_stops_at_first_track_01() {
        let tracks = vec![Some(4), Some(3), Some(2), Some(1), Some(4), Some(3)];
        assert_eq!(latest_credit_count(&tracks), 4);
    }

    #[test]
    fn latest_credit_count_includes_only_one_track() {
        let tracks = vec![Some(1), Some(4), Some(3), Some(2)];
        assert_eq!(latest_credit_count(&tracks), 1);
    }

    #[test]
    fn latest_credit_count_falls_back_when_missing() {
        let tracks = vec![Some(4), Some(3), Some(2)];
        assert_eq!(latest_credit_count(&tracks), 3);
        let tracks = vec![Some(4), Some(3), Some(2), Some(4), Some(3)];
        assert_eq!(latest_credit_count(&tracks), 4);
    }

    #[test]
    fn latest_credit_count_u8_stops_at_track_01() {
        let tracks = vec![Some(4), Some(3), Some(2), Some(1), Some(4)];
        assert_eq!(latest_credit_count_u8(&tracks), 4);
    }
}
