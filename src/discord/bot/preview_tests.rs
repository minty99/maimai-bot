use crate::discord::bot::RecentOptionalFields;

use super::{RecentRecordView, ScoreRowView};
use super::{build_mai_recent_embeds, build_mai_score_embed, build_mai_today_embed};
use dotenvy::dotenv;
use eyre::WrapErr;
use poise::serenity_prelude as serenity;
use serenity::builder::CreateMessage;

#[tokio::test]
#[ignore = "Sends a real DM to preview embed UI; requires DISCORD_BOT_TOKEN and DISCORD_USER_ID"]
async fn preview_embed_mai_score_dm() -> eyre::Result<()> {
    dotenv().ok();

    let token = std::env::var("DISCORD_BOT_TOKEN").ok();
    let user_id = std::env::var("DISCORD_USER_ID").ok();
    let (Some(token), Some(user_id)) = (token, user_id) else {
        return Ok(());
    };

    let http = serenity::Http::new(&token);
    let user_id = serenity::UserId::new(user_id.parse::<u64>().wrap_err("parse DISCORD_USER_ID")?);

    let entries = vec![
        ScoreRowView {
            chart_type: "STD".to_string(),
            diff_category: "EXPERT".to_string(),
            level: "12+".to_string(),
            internal_level: Some(12.8),
            rating_points: Some(303),
            achievement_percent: Some(99.1234),
            rank: Some("SSS".to_string()),
        },
        ScoreRowView {
            chart_type: "DX".to_string(),
            diff_category: "MASTER".to_string(),
            level: "13".to_string(),
            internal_level: None,
            rating_points: None,
            achievement_percent: Some(100.0000),
            rank: Some("SSS+".to_string()),
        },
    ];

    let embed = build_mai_score_embed("maimai-user", "Sample Song", &entries);

    let dm = user_id
        .create_dm_channel(&http)
        .await
        .wrap_err("create DM channel")?;

    let result = dm
        .send_message(&http, CreateMessage::new().embed(embed))
        .await
        .wrap_err("send DM")?;

    println!("DM sent: {}", result.id);

    Ok(())
}

#[tokio::test]
#[ignore = "Sends a real DM to preview embed UI; requires DISCORD_BOT_TOKEN and DISCORD_USER_ID"]
async fn preview_embed_mai_recent_dm() -> eyre::Result<()> {
    dotenv().ok();

    let token = std::env::var("DISCORD_BOT_TOKEN").ok();
    let user_id = std::env::var("DISCORD_USER_ID").ok();
    let (Some(token), Some(user_id)) = (token, user_id) else {
        return Ok(());
    };

    let http = serenity::Http::new(&token);
    let user_id = serenity::UserId::new(user_id.parse::<u64>().wrap_err("parse DISCORD_USER_ID")?);

    let records = vec![
        RecentRecordView {
            track: Some(1),
            played_at: Some("2026/01/20 12:34".to_string()),
            title: "Sample Song A".to_string(),
            chart_type: "STD".to_string(),
            diff_category: Some("EXPERT".to_string()),
            level: Some("12+".to_string()),
            internal_level: Some(12.8),
            rating_points: Some(303),
            achievement_percent: Some(98.7654),
            achievement_new_record: false,
            first_play: false,
            rank: Some("SS".to_string()),
        },
        RecentRecordView {
            track: Some(2),
            played_at: Some("2026/01/20 12:38".to_string()),
            title: "Sample Song B".to_string(),
            chart_type: "DX".to_string(),
            diff_category: Some("MASTER".to_string()),
            level: Some("14".to_string()),
            internal_level: None,
            rating_points: None,
            achievement_percent: Some(100.0000),
            achievement_new_record: true,
            first_play: false,
            rank: Some("SSS+".to_string()),
        },
    ];

    let optional_fields = RecentOptionalFields {
        rating: Some("1500".to_string()),
        play_count: Some("100".to_string()),
    };
    let embeds = build_mai_recent_embeds("maimai-user", &records, Some(&optional_fields), None);

    let dm = user_id
        .create_dm_channel(&http)
        .await
        .wrap_err("create DM channel")?;

    let result = dm
        .send_message(&http, CreateMessage::new().embeds(embeds))
        .await
        .wrap_err("send DM")?;

    println!("DM sent: {}", result.id);

    Ok(())
}

#[tokio::test]
#[ignore = "Sends a real DM to preview embed UI; requires DISCORD_BOT_TOKEN and DISCORD_USER_ID"]
async fn preview_embed_mai_today_dm() -> eyre::Result<()> {
    dotenv().ok();

    let token = std::env::var("DISCORD_BOT_TOKEN").ok();
    let user_id = std::env::var("DISCORD_USER_ID").ok();
    let (Some(token), Some(user_id)) = (token, user_id) else {
        return Ok(());
    };

    let http = serenity::Http::new(&token);
    let user_id = serenity::UserId::new(user_id.parse::<u64>().wrap_err("parse DISCORD_USER_ID")?);

    let embed = build_mai_today_embed(
        "maimai-user",
        "2026/01/21 04:00",
        "2026/01/22 04:00",
        2,
        6,
        1,
        1,
    );

    let dm = user_id
        .create_dm_channel(&http)
        .await
        .wrap_err("create DM channel")?;

    let result = dm
        .send_message(&http, CreateMessage::new().embed(embed))
        .await
        .wrap_err("send DM")?;

    println!("DM sent: {}", result.id);

    Ok(())
}
