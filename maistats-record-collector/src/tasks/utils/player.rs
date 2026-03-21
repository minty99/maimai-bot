use eyre::{Result, WrapErr};
use sqlx::SqlitePool;

use crate::db::{get_app_state_string, get_app_state_u32};
use crate::http_client::MaimaiClient;
use crate::tasks::utils::auth::fetch_html_with_auth_recovery;
use crate::tasks::utils::source::ExpectedPage;
use maimai_parsers::parse_player_data_html;
use models::ParsedPlayerProfile;

pub(crate) const STATE_KEY_USER_NAME: &str = "player.user_name";
pub(crate) const STATE_KEY_TOTAL_PLAY_COUNT: &str = "player.total_play_count";
pub(crate) const STATE_KEY_RATING: &str = "player.rating";
pub(crate) const STATE_KEY_CURRENT_VERSION_PLAY_COUNT: &str = "player.current_version_play_count";

pub(crate) async fn fetch_player_data_logged_in(
    client: &mut MaimaiClient,
) -> Result<ParsedPlayerProfile> {
    let url = reqwest::Url::parse("https://maimaidx-eng.com/maimai-mobile/playerData/")
        .wrap_err("parse playerData url")?;
    let html = fetch_html_with_auth_recovery(client, &url, ExpectedPage::PlayerData).await?;
    parse_player_data_html(&html).wrap_err("parse playerData html")
}

pub(crate) async fn load_stored_total_play_count(pool: &SqlitePool) -> Result<Option<u32>> {
    get_app_state_u32(pool, STATE_KEY_TOTAL_PLAY_COUNT)
        .await
        .wrap_err("load stored total play count")
}

pub(crate) async fn load_stored_player_profile(
    pool: &SqlitePool,
) -> Result<Option<ParsedPlayerProfile>> {
    let user_name = get_app_state_string(pool, STATE_KEY_USER_NAME)
        .await
        .wrap_err("load stored user name")?;
    let rating = get_app_state_u32(pool, STATE_KEY_RATING)
        .await
        .wrap_err("load stored rating")?;
    let current_version_play_count = get_app_state_u32(pool, STATE_KEY_CURRENT_VERSION_PLAY_COUNT)
        .await
        .wrap_err("load stored current version play count")?;
    let total_play_count = get_app_state_u32(pool, STATE_KEY_TOTAL_PLAY_COUNT)
        .await
        .wrap_err("load stored total play count")?;

    let (Some(user_name), Some(rating), Some(current_version_play_count), Some(total_play_count)) = (
        user_name,
        rating,
        current_version_play_count,
        total_play_count,
    ) else {
        return Ok(None);
    };

    Ok(Some(ParsedPlayerProfile {
        user_name,
        rating,
        current_version_play_count,
        total_play_count,
    }))
}

pub(crate) async fn has_incomplete_stored_player_profile(pool: &SqlitePool) -> Result<bool> {
    let user_name = get_app_state_string(pool, STATE_KEY_USER_NAME)
        .await
        .wrap_err("load stored user name")?;
    let rating = get_app_state_u32(pool, STATE_KEY_RATING)
        .await
        .wrap_err("load stored rating")?;
    let current_version_play_count = get_app_state_u32(pool, STATE_KEY_CURRENT_VERSION_PLAY_COUNT)
        .await
        .wrap_err("load stored current version play count")?;
    let total_play_count = get_app_state_u32(pool, STATE_KEY_TOTAL_PLAY_COUNT)
        .await
        .wrap_err("load stored total play count")?;

    let fields_present = [
        user_name.is_some(),
        rating.is_some(),
        current_version_play_count.is_some(),
        total_play_count.is_some(),
    ];

    Ok(fields_present.iter().any(|present| *present)
        && fields_present.iter().any(|present| !present))
}
