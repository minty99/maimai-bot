use eyre::Result;
use maimai_http_client::is_maintenance_window_now;
use models::ParsedPlayerData;
use poise::serenity_prelude as serenity;
use serenity::builder::{CreateEmbed, CreateMessage};
use tracing::{info, warn};

use super::client::BackendClient;
use super::embeds::{embed_backend_unavailable, embed_base, embed_maintenance};

pub(crate) async fn send_startup_dm(
    http: &serenity::Http,
    user_id: serenity::UserId,
    backend_client: &BackendClient,
) -> Result<()> {
    let dm_channel = user_id.create_dm_channel(http).await.map_err(|e| {
        warn!("Failed to create DM channel: {e}");
        e
    })?;

    let embed = if is_maintenance_window_now() {
        info!("Maintenance window detected, sending maintenance embed");
        embed_maintenance()
    } else {
        match backend_client.get_player().await {
            Ok(player_data) => {
                info!("Fetched player data successfully");
                embed_startup(&player_data)
            }
            Err(e) => {
                warn!("Failed to fetch player data: {e}");
                embed_backend_unavailable()
            }
        }
    };

    if let Err(e) = dm_channel
        .send_message(http, CreateMessage::new().embed(embed))
        .await
    {
        warn!("Failed to send startup DM: {e}");
    }

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
