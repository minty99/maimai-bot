mod config;
mod error;
mod rating;
mod routes;
mod state;
mod tasks;

use eyre::WrapErr;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    tracing::info!("Backend starting...");

    let config = config::BackendConfig::from_env().wrap_err("Failed to load backend config")?;

    // Create data directory if it doesn't exist (prevents permission errors on first run)
    std::fs::create_dir_all(&config.data_dir).wrap_err("Failed to create data directory")?;

    // Initialize database pool
    let db_pool = maimai_db::connect(&config.database_url)
        .await
        .wrap_err("Failed to connect to database")?;

    tracing::info!("Database connected successfully");

    tracing::info!("Running database migrations...");
    maimai_db::migrate(&db_pool)
        .await
        .wrap_err("Failed to run database migrations")?;
    tracing::info!("Database migrations completed successfully");

    // Attempt startup sync, but allow backend to start even if it fails
    // (useful for testing with invalid credentials)
    match tasks::startup::startup_sync(&db_pool, &config).await {
        Ok(_) => tracing::info!("Startup sync completed successfully"),
        Err(e) => tracing::warn!("Startup sync failed (backend will still start): {}", e),
    }

    let song_data_base_path = std::path::PathBuf::from(&config.data_dir).join("song_data");

    let song_data =
        match models::SongDataIndex::load_with_base_path(&song_data_base_path.to_string_lossy()) {
            Ok(Some(data)) => {
                tracing::info!(
                    "Song data loaded successfully from {}",
                    song_data_base_path.display()
                );
                std::sync::Arc::new(data)
            }
            Ok(None) => {
                tracing::warn!(
                    "Song data not found at {} (using empty index)",
                    song_data_base_path.display()
                );
                std::sync::Arc::new(models::SongDataIndex::empty())
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to load song data from {} (using empty index): {}",
                    song_data_base_path.display(),
                    e
                );
                std::sync::Arc::new(models::SongDataIndex::empty())
            }
        };

    let app_state = state::AppState {
        db_pool,
        config: config.clone(),
        song_data: std::sync::Arc::new(std::sync::RwLock::new(song_data)),
        song_data_base_path,
    };

    // Start background polling task
    tasks::polling::start_background_polling(app_state.clone());

    // Start song metadata builder/updater (startup + daily 07:30 KST)
    tasks::songdb::start_songdb_tasks(app_state.clone());

    let app = routes::create_routes(app_state.clone());

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = TcpListener::bind(&addr)
        .await
        .wrap_err("Failed to bind to address")?;

    tracing::info!("Server listening on {}", addr);

    axum::serve(listener, app).await.wrap_err("Server error")?;

    Ok(())
}
