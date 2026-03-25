mod config;
mod songdb;
mod tasks;

use eyre::WrapErr;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    tracing::info!("Song database generator starting...");

    let config = config::Config::from_env().wrap_err("Failed to load song database config")?;
    tasks::songdb::generate_song_database(std::path::Path::new(&config.song_data_path)).await?;
    tracing::info!("Song database generation complete");
    Ok(())
}
