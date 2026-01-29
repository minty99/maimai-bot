use eyre::{Result, WrapErr};
use poise::serenity_prelude as serenity;
use serenity::builder::CreateMessage;
use tracing::warn;

use crate::db::format_chart_type;

use super::embeds::{
    RecentOptionalFields, RecentRecordView, build_mai_recent_embeds, embed_startup, format_delta,
    rating_points_for_credit_entry,
};
use super::refresh::NetworkRefreshUpdate;
use super::types::BotData;

pub(crate) async fn send_startup_dm(
    bot_data: &BotData,
    player_data: &crate::maimai::models::ParsedPlayerData,
) -> Result<()> {
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

pub(crate) async fn send_player_update_dm(
    bot_data: &BotData,
    update: NetworkRefreshUpdate,
) -> Result<()> {
    let NetworkRefreshUpdate {
        prev_total,
        prev_rating,
        current,
        credit_entries,
    } = update;

    let http = &bot_data.discord_http;
    let dm_channel = bot_data
        .discord_user_id
        .create_dm_channel(http)
        .await
        .wrap_err("create DM channel")?;

    let optional_fields = RecentOptionalFields {
        rating: Some(format_delta(current.rating, prev_rating)),
        play_count: Some(format_delta(current.total_play_count, prev_total)),
    };
    let records = credit_entries
        .iter()
        .map(|r| RecentRecordView {
            track: r.track.map(i64::from),
            played_at: r.played_at.clone(),
            title: r.title.clone(),
            chart_type: format_chart_type(r.chart_type).to_string(),
            diff_category: r.diff_category.map(|d| d.as_str().to_string()),
            level: r.level.clone(),
            internal_level: r.diff_category.and_then(|d| {
                bot_data.song_data.as_deref().and_then(|idx| {
                    idx.internal_level(&r.title, format_chart_type(r.chart_type), d.as_str())
                })
            }),
            rating_points: rating_points_for_credit_entry(bot_data.song_data.as_deref(), r),
            achievement_percent: r.achievement_percent.map(|p| p as f64),
            achievement_new_record: r.achievement_new_record,
            first_play: r.first_play,
            rank: r.score_rank.map(|rk| rk.as_str().to_string()),
        })
        .collect::<Vec<_>>();

    let embeds = build_mai_recent_embeds(
        &current.user_name,
        &records,
        Some(&optional_fields),
        bot_data.song_data.as_deref(),
    );

    let mut attachments = Vec::new();
    if let Some(idx) = bot_data.song_data.as_deref() {
        let mut seen = std::collections::HashSet::<String>::new();
        for r in &records {
            let Some(image_name) = idx.image_name(&r.title) else {
                continue;
            };
            if !seen.insert(image_name.to_string()) {
                continue;
            }
            let path = format!("fetched_data/img/cover-m/{image_name}");
            match serenity::CreateAttachment::path(&path).await {
                Ok(att) => attachments.push(att),
                Err(e) => warn!("failed to attach cover image {path}: {e:?}"),
            }
        }
    }

    dm_channel
        .send_message(
            http,
            CreateMessage::new().add_files(attachments).embeds(embeds),
        )
        .await
        .wrap_err("send DM")?;
    Ok(())
}
