use eyre::{Result, WrapErr};
use poise::serenity_prelude as serenity;
use poise::{CreateReply, FrameworkOptions};
use reqwest::Url;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tracing::{error, info};

use crate::config::AppConfig;
use crate::db;
use crate::db::{SqlitePool, format_chart_type, format_diff, format_percent_f64, format_track};
use crate::http::MaimaiClient;
use crate::maimai::models::ParsedPlayRecord;
use crate::maimai::parse::recent::parse_recent_html;
use crate::maimai::parse::score_list::parse_scores_html;

type Context<'a> = poise::Context<'a, BotData, Error>;
type Error = eyre::Report;

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
            commands: vec![mai_record(), mai_recent()],
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

                initial_scores_sync(&bot_data)
                    .await
                    .wrap_err("startup scores sync")?;

                start_background_tasks(bot_data.clone(), ctx.cache.clone());

                poise::builtins::register_globally(ctx, &framework.options().commands)
                    .await
                    .wrap_err("register commands globally")?;

                send_startup_dm(&bot_data)
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

fn start_background_tasks(bot_data: BotData, _cache: Arc<serenity::Cache>) {
    tokio::spawn(async move {
        let mut timer = interval(Duration::from_secs(600));
        timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        info!("Background task started: periodic recent fetch (every 10 minutes)");

        loop {
            timer.tick().await;

            info!("Running periodic recent fetch...");

            if let Err(e) = periodic_recent_fetch(&bot_data).await {
                error!("Periodic recent fetch failed: {}", e);
            }
        }
    });
}

async fn periodic_recent_fetch(bot_data: &BotData) -> Result<()> {
    let mut client = MaimaiClient::new(&bot_data.config)?;
    client.login().await.wrap_err("login")?;

    let url = Url::parse("https://maimaidx-eng.com/maimai-mobile/record/")
        .wrap_err("parse record url")?;
    let bytes = client.get_bytes(&url).await.wrap_err("fetch record url")?;
    let html = String::from_utf8(bytes).wrap_err("record response is not utf-8")?;
    let entries = parse_recent_html(&html).wrap_err("parse recent html")?;

    let scraped_at = unix_timestamp();

    let new_records = find_new_records(&bot_data.db, scraped_at, &entries)
        .await
        .wrap_err("find new records")?;

    if !new_records.is_empty() {
        info!("Found {} new records", new_records.len());
        send_new_records_dm(bot_data, &new_records)
            .await
            .wrap_err("send new records DM")?;
    } else {
        info!("No new records found");
    }

    Ok(())
}

async fn find_new_records(
    pool: &SqlitePool,
    scraped_at: i64,
    entries: &[ParsedPlayRecord],
) -> Result<Vec<ParsedPlayRecord>> {
    let mut new_records = Vec::new();

    for entry in entries {
        let Some(playlog_idx) = entry.playlog_idx.as_ref() else {
            continue;
        };

        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM playlogs WHERE playlog_idx = ?)")
                .bind(playlog_idx)
                .fetch_one(pool)
                .await
                .wrap_err("check playlog existence")?;

        if !exists {
            new_records.push(entry.clone());

            crate::db::upsert_playlogs(pool, scraped_at, std::slice::from_ref(entry))
                .await
                .wrap_err("upsert playlog")?;
        }
    }

    Ok(new_records)
}

async fn send_new_records_dm(bot_data: &BotData, records: &[ParsedPlayRecord]) -> Result<()> {
    let http = &bot_data.discord_http;

    let dm_channel = bot_data
        .discord_user_id
        .create_dm_channel(http)
        .await
        .wrap_err("create DM channel")?;

    let message = format_new_records(records);

    dm_channel.say(http, message).await.wrap_err("send DM")?;

    info!("Sent DM with {} new records", records.len());

    Ok(())
}

async fn send_startup_dm(bot_data: &BotData) -> Result<()> {
    let http = &bot_data.discord_http;
    let dm_channel = bot_data
        .discord_user_id
        .create_dm_channel(http)
        .await
        .wrap_err("create DM channel")?;

    let message = format!("✅ maimai-bot started\n- unix_ts: {}", unix_timestamp());
    dm_channel.say(http, message).await.wrap_err("send DM")?;
    Ok(())
}

fn format_new_records(records: &[ParsedPlayRecord]) -> String {
    let mut lines = vec!["🎵 **New Records Detected!**".to_string(), String::new()];

    for record in records {
        lines.push(format_playlog_record(record));
        lines.push(format_playlog_stats(record));
        lines.push(String::new());
    }

    lines.join("\n")
}

/// Get song records by song title or key
#[poise::command(slash_command, rename = "mai-record")]
async fn mai_record(
    ctx: Context<'_>,
    #[description = "Song title or key to search for"] search: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let rows = sqlx::query_as::<_, (String, String, String, Option<f64>, Option<String>)>(
        r#"
        SELECT
            s.title,
            sc.chart_type,
            CASE sc.diff
                WHEN 0 THEN 'BASIC'
                WHEN 1 THEN 'ADVANCED'
                WHEN 2 THEN 'EXPERT'
                WHEN 3 THEN 'MASTER'
                WHEN 4 THEN 'Re:MASTER'
                ELSE 'Unknown'
            END as diff_name,
            sc.achievement_percent,
            sc.rank
        FROM scores sc
        JOIN songs s ON sc.song_key = s.song_key
        WHERE s.title LIKE ? OR s.song_key = ?
        ORDER BY sc.chart_type, sc.diff
        "#,
    )
    .bind(format!("%{}%", search))
    .bind(&search)
    .fetch_all(&ctx.data().db)
    .await
    .map_err(|e| eyre::eyre!("query failed: {}", e))?;

    if rows.is_empty() {
        ctx.say(format!("No records found for '{}'", search))
            .await?;
        return Ok(());
    }

    let mut lines = vec![format!("📊 Records for '{}'", search), String::new()];

    for (title, chart_type, diff_name, achievement, rank) in rows {
        lines.push(format!(
            "**{} [{}] {}**: {} - {}",
            title,
            chart_type,
            diff_name,
            format_percent_f64(achievement),
            rank.unwrap_or_else(|| "N/A".to_string())
        ));
    }

    ctx.say(lines.join("\n")).await?;

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
            pl.achievement_percent,
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
        ctx.say("No recent records found".to_string()).await?;
        return Ok(());
    }

    let take = latest_credit_count(&rows.iter().map(|row| row.2).collect::<Vec<_>>());
    let mut lines = vec![
        format!("🕐 Recent 1 Credit ({} plays)", take),
        String::new(),
    ];

    for (title, chart_type, track, played_at, achievement, rank) in rows.into_iter().take(take) {
        lines.push(format!(
            "**{}** [{}] - {} @ {}",
            title,
            chart_type,
            format_track(track),
            played_at.unwrap_or_else(|| "N/A".to_string())
        ));
        lines.push(format!(
            "📊 {} - {}",
            format_percent_f64(achievement),
            rank.unwrap_or_else(|| "N/A".to_string())
        ));
        lines.push(String::new());
    }

    ctx.say(lines.join("\n")).await?;

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

fn format_playlog_record(record: &ParsedPlayRecord) -> String {
    format!(
        "**{}** [{}] {} - {}",
        record.title,
        format_chart_type(record.chart_type),
        format_diff(record.diff),
        record.played_at.as_deref().unwrap_or("N/A")
    )
}

fn format_playlog_stats(record: &ParsedPlayRecord) -> String {
    format!(
        "📊 {}  🏆 {}  🎯 {}  👥 {}  💫 {}",
        format_percent_f64(record.achievement_percent.map(|p| p as f64)),
        record.score_rank.as_deref().unwrap_or("N/A"),
        record.fc.as_deref().unwrap_or("N/A"),
        record.sync.as_deref().unwrap_or("N/A"),
        record
            .dx_score
            .and_then(|s| record.dx_score_max.map(|m| format!("{}/{}", s, m)))
            .unwrap_or_else(|| "N/A".to_string())
    )
}

#[cfg(test)]
mod tests {
    use super::latest_credit_count;

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
}
