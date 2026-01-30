use eyre::{Result, WrapErr};
use poise::serenity_prelude as serenity;
use serenity::builder::{CreateEmbed, CreateMessage};

use super::embeds::embed_startup;
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

pub(crate) async fn send_embed_dm(bot_data: &BotData, embed: CreateEmbed) -> Result<()> {
    let http = &bot_data.discord_http;
    let dm_channel = bot_data
        .discord_user_id
        .create_dm_channel(http)
        .await
        .wrap_err("create DM channel")?;

    dm_channel
        .send_message(http, CreateMessage::new().embed(embed))
        .await
        .wrap_err("send DM")?;
    Ok(())
}
