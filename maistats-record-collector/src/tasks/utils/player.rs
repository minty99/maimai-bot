use eyre::{Result, WrapErr};
use sqlx::SqlitePool;

use crate::db::get_app_state_u32;
use crate::http_client::MaimaiClient;
use crate::tasks::utils::auth::fetch_html_with_auth_recovery;
use crate::tasks::utils::source::ExpectedPage;
use maimai_parsers::parse_player_data_html;
use models::ParsedPlayerProfile;

pub(crate) const STATE_KEY_TOTAL_PLAY_COUNT: &str = "player.total_play_count";
pub(crate) const STATE_KEY_RATING: &str = "player.rating";

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
