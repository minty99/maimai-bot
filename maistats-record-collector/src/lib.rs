pub(crate) mod config;
pub mod db;
pub(crate) mod error;
pub(crate) mod http_client;
pub(crate) mod routes;
pub(crate) mod state;
pub mod tasks;

use eyre::WrapErr;
use std::net::SocketAddr;
use tokio::net::TcpListener;

pub async fn run_server() -> eyre::Result<()> {
    tracing::info!("Record collector server starting...");

    let config = config::RecordCollectorConfig::from_env()
        .wrap_err("Failed to load record collector config")?;

    std::fs::create_dir_all(&config.data_dir).wrap_err("Failed to create data directory")?;

    let db_pool = db::connect(&config.database_url)
        .await
        .wrap_err("Failed to connect to database")?;

    tracing::info!("Database connected successfully");

    tracing::info!("Running database migrations...");
    db::migrate(&db_pool)
        .await
        .wrap_err("Failed to run database migrations")?;
    tracing::info!("Database migrations completed successfully");

    match tasks::startup::startup_sync(&db_pool, &config).await {
        Ok(report) => tracing::info!(
            "Startup sync completed: maintenance_skip={} seeded={} seeded_rows={} recent_present={}",
            report.skipped_for_maintenance,
            report.seeded,
            report.seeded_rows_written,
            report.recent_outcome.is_some()
        ),
        Err(e) => tracing::warn!("Startup sync failed (server will still start): {e:#}"),
    }

    let app_state = state::AppState {
        db_pool,
        config: config.clone(),
    };

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
