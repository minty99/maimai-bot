use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

use eyre::WrapErr;
use models::{SongDataIndex, SongDataRoot, SongDataSong};

#[derive(Clone)]
pub struct AppState {
    pub song_data: Arc<RwLock<SongDataIndex>>,
    pub song_data_root: Arc<RwLock<Vec<SongDataSong>>>,
    pub song_data_base_path: PathBuf,
    pub song_data_loaded: Arc<AtomicBool>,
}

impl AppState {
    pub fn reload_song_data(&self) -> eyre::Result<()> {
        let data_path = self.song_data_base_path.join("data.json");
        let (root, index, loaded) = load_song_data(&data_path)?;

        {
            let mut song_data = self.song_data.write().unwrap();
            *song_data = index;
        }

        {
            let mut song_data_root = self.song_data_root.write().unwrap();
            *song_data_root = root.songs;
        }

        self.song_data_loaded.store(loaded, Ordering::Relaxed);

        Ok(())
    }
}

pub fn load_song_data(path: &Path) -> eyre::Result<(SongDataRoot, SongDataIndex, bool)> {
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
