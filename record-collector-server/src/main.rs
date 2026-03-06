mod config;
mod db;
mod error;
mod http_client;
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

    tracing::info!("Record collector server starting...");

    let config = config::RecordCollectorConfig::from_env()
        .wrap_err("Failed to load record collector config")?;

    // Create data directory if it doesn't exist (prevents permission errors on first run)
    std::fs::create_dir_all(&config.data_dir).wrap_err("Failed to create data directory")?;

    // Initialize database pool
    let db_pool = db::connect(&config.database_url)
        .await
        .wrap_err("Failed to connect to database")?;

    tracing::info!("Database connected successfully");

    tracing::info!("Running database migrations...");
    db::migrate(&db_pool)
        .await
        .wrap_err("Failed to run database migrations")?;
    tracing::info!("Database migrations completed successfully");

    // Attempt startup sync, but allow server to start even if it fails
    // (useful for testing with invalid credentials)
    match tasks::startup::startup_sync(&db_pool, &config).await {
        Ok(report) => tracing::info!(
            "Startup sync completed: maintenance_skip={} seeded={} seeded_rows={} recent_present={}",
            report.skipped_for_maintenance,
            report.seeded_scores.seeded,
            report.seeded_scores.rows_written,
            report.recent_outcome.is_some()
        ),
        Err(e) => tracing::warn!("Startup sync failed (server will still start): {e:#}"),
    }

    let app_state = state::AppState {
        db_pool,
        config: config.clone(),
    };

    // Start background polling task
    tasks::polling::start_background_polling(app_state.clone());

    let app = routes::create_routes(app_state.clone());

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = TcpListener::bind(&addr)
        .await
        .wrap_err("Failed to bind to address")?;

    tracing::info!("Server listening on {}", addr);

    axum::serve(listener, app).await.wrap_err("Server error")?;

    Ok(())
}
