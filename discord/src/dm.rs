use eyre::{Result, WrapErr};
use poise::serenity_prelude as serenity;
use serenity::builder::{CreateEmbed, CreateMessage};
use models::ParsedPlayerData;

use super::embeds::embed_base;
use super::client::BackendClient;

pub(crate) async fn send_startup_dm(
    http: &serenity::Http,
    user_id: serenity::UserId,
    backend_client: &BackendClient,
) -> Result<()> {
    let player_data = backend_client.get_player().await?;

    let dm_channel = user_id
        .create_dm_channel(http)
        .await
        .wrap_err("create DM channel")?;

    let embed = embed_startup(&player_data);

    dm_channel
        .send_message(http, CreateMessage::new().embed(embed))
        .await
        .wrap_err("send startup DM")?;

    Ok(())
}

fn embed_startup(player_data: &ParsedPlayerData) -> CreateEmbed {
    let mut embed = embed_base(&format!("Welcome, {}!", player_data.user_name));
    embed = embed.description(format!(
        "**Rating**: {}\n**Total Plays**: {}",
        player_data.rating, player_data.total_play_count
    ));
    embed
}
