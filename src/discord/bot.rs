use std::sync::Arc;

use eyre::{Result, WrapErr};
use poise::serenity_prelude as serenity;
use poise::{CreateReply, FrameworkOptions};
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use crate::config::AppConfig;
use crate::db;
use crate::http::MaimaiClient;
use crate::http::is_maintenance_window_now;
use crate::song_data::SongDataIndex;

mod commands;
mod dm;
mod embeds;
mod refresh;
mod types;
mod util;

pub use types::BotData;

pub(crate) use embeds::{
    RecentOptionalFields, RecentRecordView, ScoreRowView, build_mai_recent_embeds,
    build_mai_score_embed, build_mai_today_embed, embed_base, format_level_with_internal,
};
pub(crate) use refresh::sync_from_network_without_discord;
pub(crate) use util::{latest_credit_len, normalize_for_match, top_title_matches};

use commands::{mai_rating, mai_recent, mai_score, mai_today, mai_today_detail};
use dm::send_startup_dm;
use refresh::{
    fetch_player_data, initial_recent_sync, initial_scores_sync, persist_play_counts,
    should_sync_scores, start_background_tasks,
};

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

    let song_data = match SongDataIndex::load_from_default_locations() {
        Ok(v) => v.map(Arc::new),
        Err(e) => {
            warn!("failed to load song data (non-fatal): {e:?}");
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
            commands: vec![
                mai_score(),
                mai_recent(),
                mai_today(),
                mai_today_detail(),
                mai_rating(),
            ],
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
        use super::embeds::{format_delta, rating_points_for_credit_entry};
        use super::{RecentOptionalFields, build_mai_recent_embeds};
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

        let credit_entries = [
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

        let optional_fields = RecentOptionalFields {
            rating: Some(format_delta(player.rating, Some(12340))),
            play_count: Some(format_delta(player.total_play_count, Some(889))),
        };

        let records = credit_entries
            .iter()
            .map(|r| super::RecentRecordView {
                track: r.track.map(i64::from),
                played_at: r.played_at.clone(),
                title: r.title.clone(),
                chart_type: crate::db::format_chart_type(r.chart_type).to_string(),
                diff_category: r.diff_category.map(|d| d.as_str().to_string()),
                level: r.level.clone(),
                internal_level: None,
                rating_points: rating_points_for_credit_entry(None, r),
                achievement_percent: r.achievement_percent.map(|p| p as f64),
                achievement_new_record: r.achievement_new_record,
                first_play: r.first_play,
                rank: r.score_rank.map(|rk| rk.as_str().to_string()),
            })
            .collect::<Vec<_>>();

        let embeds =
            build_mai_recent_embeds(&player.user_name, &records, Some(&optional_fields), None);

        let result = dm
            .send_message(&http, CreateMessage::new().embeds(embeds))
            .await
            .wrap_err("send DM")?;

        println!("DM sent: {}", result.id);

        Ok(())
    }
}
