mod config;
mod error;
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

    let app_state = state::AppState { db_pool, config: config.clone() };

    // Start background polling task
    tasks::polling::start_background_polling(app_state.clone());

    let app = routes::create_routes(app_state.clone());

    let addr = SocketAddr::from(([127, 0, 0, 1], config.port));
    let listener = TcpListener::bind(&addr)
        .await
        .wrap_err("Failed to bind to address")?;
    
    tracing::info!("Server listening on {}", addr);

    axum::serve(listener, app)
        .await
        .wrap_err("Server error")?;

    Ok(())
}
