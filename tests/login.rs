use std::path::PathBuf;

use maimai_bot::config::AppConfig;
use maimai_bot::http::MaimaiClient;

// Live test that actually logs in to maimaidx-eng.com using SEGA_ID/SEGA_PASSWORD.
// Run: `cargo test -- --ignored --nocapture`
#[tokio::test]
#[ignore]
async fn sega_login_works() -> eyre::Result<()> {
    dotenvy::dotenv().ok();

    let sega_id = std::env::var("SEGA_ID")?;
    let sega_password = std::env::var("SEGA_PASSWORD")?;

    let discord_bot_token = std::env::var("DISCORD_BOT_TOKEN").ok();
    let discord_user_id = std::env::var("DISCORD_USER_ID").ok();

    let temp_dir = tempfile::tempdir()?;
    let cookie_path: PathBuf = temp_dir.path().join("cookies.json");

    let config = AppConfig {
        sega_id,
        sega_password,
        data_dir: temp_dir.path().to_path_buf(),
        cookie_path,
        discord_bot_token,
        discord_user_id,
    };

    let mut client = MaimaiClient::new(&config)?;
    if let Err(err) = client.login().await {
        let msg = err.to_string();
        if msg.contains("/maimai-mobile/error/") || msg.contains("ERROR CODE") {
            eprintln!("skipping live login test (site appears unavailable): {msg}");
            return Ok(());
        }
        return Err(err);
    }

    let logged_in = client.check_logged_in().await?;
    eyre::ensure!(logged_in, "expected logged_in=true after login()");

    Ok(())
}
