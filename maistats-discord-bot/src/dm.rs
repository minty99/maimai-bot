use eyre::Result;
use poise::serenity_prelude as serenity;
use serenity::builder::CreateMessage;

use crate::embeds::{embed_registration_confirmation, embed_startup_summary};

async fn send_dm(
    http: &serenity::Http,
    user_id: serenity::UserId,
    embed: serenity::CreateEmbed,
) -> Result<()> {
    let dm_channel = user_id.create_dm_channel(http).await?;
    dm_channel
        .send_message(http, CreateMessage::new().embed(embed))
        .await?;
    Ok(())
}

pub(crate) async fn send_developer_startup_dm(
    http: &serenity::Http,
    user_id: serenity::UserId,
    registered_url_count: i64,
) -> Result<()> {
    send_dm(http, user_id, embed_startup_summary(registered_url_count)).await
}

pub(crate) async fn send_registration_confirmation_dm(
    http: &serenity::Http,
    user_id: serenity::UserId,
    player_name: &str,
    url: &str,
) -> Result<()> {
    send_dm(
        http,
        user_id,
        embed_registration_confirmation(player_name, url),
    )
    .await
}
