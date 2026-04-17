use eyre::WrapErr;
use poise::serenity_prelude as serenity;
use poise::{CreateReply, FrameworkOptions};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::{info, warn};

mod chart_links;
mod commands;
mod config;
mod db;
mod dm;
mod embeds;
mod emoji;
mod plot;
mod updown;

use config::DiscordConfig;
use emoji::MaimaiStatusEmojis;
use maimai_client::SongDatabaseClient;

#[derive(Debug, Clone)]
pub(crate) struct BotData {
    pub(crate) db_pool: db::SqlitePool,
    pub(crate) dev_user_id: serenity::UserId,
    pub(crate) discord_http: std::sync::Arc<serenity::Http>,
    pub(crate) song_database_client: SongDatabaseClient,
    pub(crate) status_emojis: MaimaiStatusEmojis,
    pub(crate) version_warning_cache: Arc<Mutex<HashMap<String, i64>>>,
    pub(crate) updown_in_flight: updown::UpdownInFlightLocks,
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let config = DiscordConfig::from_env()?;

    std::fs::create_dir_all(&config.data_dir).wrap_err("create bot data directory")?;

    let discord_bot_token = config.bot_token.clone();
    let discord_http = std::sync::Arc::new(serenity::Http::new(&discord_bot_token));
    let dev_user_id = serenity::UserId::new(
        config
            .dev_user_id
            .parse::<u64>()
            .wrap_err("parse DISCORD_DEV_USER_ID")?,
    );

    let db_pool = db::connect(&config.database_url).await?;
    db::migrate(&db_pool).await?;

    let song_database_client = SongDatabaseClient::new(config.song_database_url.clone())?;

    let bot_data = BotData {
        db_pool,
        dev_user_id,
        discord_http,
        song_database_client,
        status_emojis: MaimaiStatusEmojis::default(),
        version_warning_cache: Arc::new(Mutex::new(HashMap::new())),
        updown_in_flight: updown::new_in_flight_locks(),
    };

    let framework = poise::Framework::builder()
        .options(FrameworkOptions {
            prefix_options: Default::default(),
            commands: vec![
                commands::how_to_use(),
                commands::register(),
                commands::mai_score(),
                commands::mai_song_info(),
                commands::mai_recent(),
                commands::mai_today(),
                commands::mai_updown(),
                commands::mai_plot(),
            ],
            event_handler: |ctx, event, _framework, data| {
                Box::pin(updown::handle_event(ctx, event, data))
            },
            on_error: |error: poise::FrameworkError<
                '_,
                BotData,
                Box<dyn std::error::Error + Send + Sync>,
            >| {
                Box::pin(async move {
                    match error {
                        poise::FrameworkError::Command { error, ctx, .. } => {
                            tracing::error!(
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
                            tracing::error!("Argument parse error: {:?}", error);
                            let _ = ctx
                                .send(
                                    CreateReply::default()
                                        .content("Invalid arguments")
                                        .ephemeral(true),
                                )
                                .await;
                        }
                        _ => {
                            tracing::error!("Framework error: {:?}", error);
                        }
                    }
                })
            },
            ..Default::default()
        })
        .setup(move |ctx, ready, framework| {
            let bot_data = bot_data.clone();
            Box::pin(async move {
                info!("Bot started as {}", ctx.cache.current_user().name);
                bot_data
                    .discord_http
                    .set_application_id(ready.application.id);

                poise::builtins::register_globally(ctx, &framework.options().commands)
                    .await
                    .wrap_err("register commands globally")?;

                let status_emojis =
                    match emoji::sync_application_emojis(bot_data.discord_http.as_ref()).await {
                        Ok(emojis) => emojis,
                        Err(error) => {
                            warn!("Status emoji sync failed: {error:?}");
                            MaimaiStatusEmojis::default()
                        }
                    };

                let registration_count = db::count_registrations(&bot_data.db_pool).await?;
                if let Err(e) = dm::send_developer_startup_dm(
                    &bot_data.discord_http,
                    bot_data.dev_user_id,
                    registration_count,
                )
                .await
                {
                    warn!("Developer startup DM failed: {e}");
                }

                Ok(BotData {
                    status_emojis,
                    ..bot_data
                })
            })
        })
        .build();

    let intents =
        serenity::GatewayIntents::GUILDS | serenity::GatewayIntents::GUILD_MESSAGE_REACTIONS;

    let mut client = serenity::Client::builder(&discord_bot_token, intents)
        .framework(framework)
        .await
        .wrap_err("create Discord client")?;

    info!("Starting Discord bot...");
    client.start().await.wrap_err("client error")?;

    Ok(())
}
