use eyre::WrapErr;
use models::config::AppConfig;
use reqwest::Url;

#[path = "../http_client.rs"]
mod http_client;

use http_client::MaimaiClient;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    dotenvy::dotenv().ok();

    let url = std::env::args()
        .nth(1)
        .ok_or_else(|| eyre::eyre!("usage: fetch_page <url> <out_path>"))?;
    let out_path = std::env::args()
        .nth(2)
        .ok_or_else(|| eyre::eyre!("usage: fetch_page <url> <out_path>"))?;

    let sega_id = std::env::var("SEGA_ID").wrap_err("missing SEGA_ID")?;
    let sega_password = std::env::var("SEGA_PASSWORD").wrap_err("missing SEGA_PASSWORD")?;
    let data_dir =
        std::path::PathBuf::from(std::env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string()));
    std::fs::create_dir_all(&data_dir).wrap_err("create data dir")?;
    let cookie_path =
        std::env::temp_dir().join(format!("maistats-cookies-{}.json", std::process::id()));

    let app_config = AppConfig {
        sega_id,
        sega_password,
        data_dir,
        cookie_path,
        discord_bot_token: None,
        discord_user_id: None,
    };

    let mut client = MaimaiClient::new(&app_config).wrap_err("create maimai client")?;
    client
        .ensure_logged_in()
        .await
        .wrap_err("ensure logged in")?;

    let parsed_url = Url::parse(&url).wrap_err("parse url")?;
    let response = client
        .get_response(&parsed_url)
        .await
        .wrap_err("fetch response")?;
    std::fs::write(&out_path, &response.body).wrap_err("write output file")?;

    println!(
        "wrote {} bytes from {} to {}",
        response.body.len(),
        response.final_url,
        out_path
    );
    Ok(())
}
