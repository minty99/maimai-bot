mod config;
mod error;
mod rating;
mod routes;
mod state;
mod tasks;

use eyre::WrapErr;
use std::net::SocketAddr;
use std::path::PathBuf;
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

    // Initialize database pool
    let db_pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await
        .wrap_err("Failed to connect to database")?;

    tracing::info!("Database connected successfully");

    tracing::info!("Running database migrations...");
    sqlx::migrate!("./migrations")
        .run(&db_pool)
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
                Some(std::sync::Arc::new(data))
            }
            Ok(None) => {
                tracing::warn!(
                    "Song data not found at {} (non-fatal)",
                    song_data_base_path.display()
                );
                None
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to load song data from {} (non-fatal): {}",
                    song_data_base_path.display(),
                    e
                );
                None
            }
        };

    let app_state = state::AppState {
        db_pool,
        config: config.clone(),
        song_data,
        song_data_base_path,
    };

    // Start background polling task
    tasks::polling::start_background_polling(app_state.clone());

    // Start song metadata builder/updater (startup + daily 07:30 KST)
    tasks::songdb::start_songdb_tasks(PathBuf::from(&config.data_dir));

    let app = routes::create_routes(app_state.clone());

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = TcpListener::bind(&addr)
        .await
        .wrap_err("Failed to bind to address")?;

    tracing::info!("Server listening on {}", addr);

    axum::serve(listener, app).await.wrap_err("Server error")?;

    Ok(())
}
