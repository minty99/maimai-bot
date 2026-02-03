mod config;
mod error;
mod routes;
mod state;

use eyre::WrapErr;
use models::{SongDataIndex, SongDataRoot};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
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

    let song_data_path = PathBuf::from(&config.song_data_path);
    let song_data_base_path = song_data_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();

    let (song_data_root, song_data_index, song_data_loaded) = match load_song_data(&song_data_path)
    {
        Ok((root, index, loaded)) => {
            if loaded {
                tracing::info!(
                    "Song data loaded successfully from {}",
                    song_data_path.display()
                );
            } else {
                tracing::warn!(
                    "Song data not found at {} (using empty index)",
                    song_data_path.display()
                );
            }
            (root, index, loaded)
        }
        Err(e) => {
            tracing::warn!(
                "Failed to load song data from {} (using empty index): {}",
                song_data_path.display(),
                e
            );
            (
                SongDataRoot { songs: Vec::new() },
                SongDataIndex::empty(),
                false,
            )
        }
    };

    let app_state = state::AppState {
        song_data: Arc::new(RwLock::new(song_data_index)),
        song_data_root: Arc::new(song_data_root.songs),
        song_data_base_path,
        song_data_loaded,
    };

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

fn load_song_data(path: &Path) -> eyre::Result<(SongDataRoot, SongDataIndex, bool)> {
    if !path.exists() {
        return Ok((
            SongDataRoot { songs: Vec::new() },
            SongDataIndex::empty(),
            false,
        ));
    }

    let bytes = std::fs::read(path).wrap_err("read song data")?;
    let root: SongDataRoot = serde_json::from_slice(&bytes).wrap_err("parse song data")?;
    let index_root: SongDataRoot =
        serde_json::from_slice(&bytes).wrap_err("parse song data for index")?;
    let index = SongDataIndex::from_root(index_root);

    Ok((root, index, true))
}

async fn shutdown_signal() {
    use tokio::signal::unix::{signal, SignalKind};

    let mut sigterm = signal(SignalKind::terminate()).expect("install SIGTERM handler");
    tokio::select! {
        _ = sigterm.recv() => {},
        _ = tokio::signal::ctrl_c() => {},
    }
}
