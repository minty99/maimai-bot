mod config;
mod error;
mod rating;
mod routes;
mod state;
mod tasks;

use eyre::WrapErr;
use tracing_subscriber::EnvFilter;
use std::net::SocketAddr;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    tracing::info!("Backend starting...");

    let config = config::BackendConfig::from_env()
        .wrap_err("Failed to load backend config")?;

    // Initialize database pool
    let db_pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await
        .wrap_err("Failed to connect to database")?;

    tracing::info!("Database connected successfully");

    // Attempt startup sync, but allow backend to start even if it fails
    // (useful for testing with invalid credentials)
    match tasks::startup::startup_sync(&db_pool, &config).await {
        Ok(_) => tracing::info!("Startup sync completed successfully"),
        Err(e) => tracing::warn!("Startup sync failed (backend will still start): {}", e),
    }

    if !std::path::Path::new(&config.fetched_data_path).exists() {
        tracing::warn!(
            "fetched_data directory not found at '{}' - song metadata and album covers will be unavailable",
            config.fetched_data_path
        );
    }

    let song_data = match models::SongDataIndex::load_with_base_path(&config.fetched_data_path) {
        Ok(Some(data)) => {
            tracing::info!("Song data loaded successfully from {}", config.fetched_data_path);
            Some(std::sync::Arc::new(data))
        }
        Ok(None) => {
            tracing::warn!("Song data not found at {} (non-fatal)", config.fetched_data_path);
            None
        }
        Err(e) => {
            tracing::warn!("Failed to load song data from {} (non-fatal): {}", config.fetched_data_path, e);
            None
        }
    };

    let app_state = state::AppState { 
        db_pool, 
        config: config.clone(),
        song_data,
        fetched_data_path: config.fetched_data_path.clone(),
    };

    // Start background polling task
    tasks::polling::start_background_polling(app_state.clone());

    let app = routes::create_routes(app_state.clone());

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = TcpListener::bind(&addr)
        .await
        .wrap_err("Failed to bind to address")?;
    
    tracing::info!("Server listening on {}", addr);

    axum::serve(listener, app)
        .await
        .wrap_err("Server error")?;

    Ok(())
}
