mod config;
mod error;
mod routes;
mod state;
mod tasks;

use eyre::WrapErr;
use std::net::SocketAddr;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, RwLock};
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    tracing::info!("Song info server starting...");

    let config = config::Config::from_env().wrap_err("Failed to load song info config")?;

    let song_data_base_path = std::path::PathBuf::from(&config.song_data_path);
    let song_data_json_path = song_data_base_path.join("data.json");

    let (song_data_root, song_data_index, song_data_loaded) =
        match state::load_song_data(&song_data_json_path) {
            Ok((root, index, loaded)) => {
                if loaded {
                    tracing::info!(
                        "Song data loaded successfully from {}",
                        song_data_json_path.display()
                    );
                } else {
                    tracing::warn!(
                        "Song data not found at {} (using empty index)",
                        song_data_json_path.display()
                    );
                }
                (root, index, loaded)
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to load song data from {} (using empty index): {}",
                    song_data_json_path.display(),
                    e
                );
                (
                    models::SongDataRoot { songs: Vec::new() },
                    models::SongDataIndex::empty(),
                    false,
                )
            }
        };

    let app_state = state::AppState {
        song_data: Arc::new(RwLock::new(song_data_index)),
        song_data_root: Arc::new(RwLock::new(song_data_root.songs)),
        song_data_base_path,
        song_data_loaded: Arc::new(AtomicBool::new(song_data_loaded)),
    };

    tasks::songdb::start_songdb_tasks(app_state.clone());

    let app = routes::create_router(app_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = TcpListener::bind(&addr)
        .await
        .wrap_err("Failed to bind to address")?;

    tracing::info!("Server listening on {}", addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .wrap_err("Server error")?;

    Ok(())
}

async fn shutdown_signal() {
    use tokio::signal::unix::{signal, SignalKind};

    let mut sigterm = signal(SignalKind::terminate()).expect("install SIGTERM handler");
    tokio::select! {
        _ = sigterm.recv() => {},
        _ = tokio::signal::ctrl_c() => {},
    }
}
