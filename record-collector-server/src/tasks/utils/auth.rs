use eyre::{Result, WrapErr};
use reqwest::Url;

use crate::config::RecordCollectorConfig;
use crate::http_client::MaimaiClient;
use maimai_auth::intl;
use models::config::AppConfig;

#[derive(Debug, Clone)]
pub(crate) enum ExpectedPage {
    PlayerData,
    Recent,
    ScoresList { diff: u8 },
    PlaylogDetail { idx: String },
    MusicDetail { idx: String },
}

pub(crate) fn to_app_config(config: &RecordCollectorConfig) -> AppConfig {
    use std::path::PathBuf;

    let data_dir = PathBuf::from(&config.data_dir);
    let cookie_path =
        std::env::temp_dir().join(format!("maistats-cookies-{}.json", std::process::id()));

    AppConfig {
        sega_id: config.sega_id.clone(),
        sega_password: config.sega_password.clone(),
        data_dir,
        cookie_path,
        discord_bot_token: None,
        discord_user_id: None,
    }
}

pub(crate) fn build_client(config: &RecordCollectorConfig) -> Result<MaimaiClient> {
    let app_config = to_app_config(config);
    MaimaiClient::new(&app_config).wrap_err("create HTTP client")
}

pub(crate) async fn ensure_session(client: &mut MaimaiClient) -> Result<()> {
    client.ensure_logged_in().await.wrap_err("ensure logged in")
}

pub(crate) async fn fetch_html_with_auth_recovery(
    client: &mut MaimaiClient,
    url: &Url,
    expected_page: ExpectedPage,
) -> Result<String> {
    let first = client
        .get_response(url)
        .await
        .wrap_err_with(|| format!("fetch {}", expected_page_label(&expected_page)))?;
    let first_html = String::from_utf8(first.body).wrap_err("response is not utf-8")?;
    if !intl::looks_like_login_or_expired(&first.final_url, &first_html) {
        return Ok(first_html);
    }

    client
        .login()
        .await
        .wrap_err("re-login after auth expiry")?;

    let second = client
        .get_response(url)
        .await
        .wrap_err_with(|| format!("retry fetch {}", expected_page_label(&expected_page)))?;
    let second_html = String::from_utf8(second.body).wrap_err("retry response is not utf-8")?;
    if intl::looks_like_login_or_expired(&second.final_url, &second_html) {
        return Err(eyre::eyre!(
            "{} still looks unauthenticated after re-login: {}",
            expected_page_label(&expected_page),
            second.final_url
        ));
    }

    Ok(second_html)
}

fn expected_page_label(expected_page: &ExpectedPage) -> String {
    match expected_page {
        ExpectedPage::PlayerData => "playerData page".to_string(),
        ExpectedPage::Recent => "recent page".to_string(),
        ExpectedPage::ScoresList { diff } => format!("scores list page (diff={diff})"),
        ExpectedPage::PlaylogDetail { idx } => format!("playlogDetail page (idx={idx})"),
        ExpectedPage::MusicDetail { idx } => format!("musicDetail page (idx={idx})"),
    }
}
