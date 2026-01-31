use eyre::WrapErr;
use poise::serenity_prelude as serenity;
use poise::{CreateReply, FrameworkOptions};
use tracing::info;

mod config;
mod client;
mod commands;
mod embeds;
mod dm;

use config::DiscordConfig;
use client::BackendClient;

#[derive(Debug)]
pub struct BotData {
    pub config: DiscordConfig,
    pub discord_user_id: serenity::UserId,
    pub discord_http: std::sync::Arc<serenity::Http>,
    pub backend_client: BackendClient,
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = DiscordConfig::from_env()?;
    
    let discord_bot_token = config.bot_token.clone();
    let discord_user_id_str = config.user_id.clone();

    let discord_http = std::sync::Arc::new(serenity::Http::new(&discord_bot_token));

    let discord_user_id = serenity::UserId::new(
        discord_user_id_str
            .parse::<u64>()
            .wrap_err("parse DISCORD_USER_ID")?,
    );

    let backend_client = BackendClient::new(config.backend_url.clone())?;

    info!("Waiting for backend to be ready...");
    backend_client.health_check_with_retry().await?;

    let bot_data = BotData {
        config,
        discord_user_id,
        discord_http,
        backend_client,
    };

    let framework = poise::Framework::builder()
        .options(FrameworkOptions {
            prefix_options: Default::default(),
            commands: vec![
                commands::mai_score(),
                commands::mai_recent(),
                commands::mai_today(),
                commands::mai_today_detail(),
                commands::mai_rating(),
            ],
            on_error: |error: poise::FrameworkError<'_, BotData, Box<dyn std::error::Error + Send + Sync>>| {
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
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                info!("Bot started as {}", ctx.cache.current_user().name);

                poise::builtins::register_globally(ctx, &framework.options().commands)
                    .await
                    .wrap_err("register commands globally")?;

                dm::send_startup_dm(&bot_data.discord_http, bot_data.discord_user_id, &bot_data.backend_client)
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
