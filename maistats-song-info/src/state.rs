use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

use eyre::WrapErr;
use models::{SongCatalog, SongCatalogSong, SongInternalLevelIndex};

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) song_data: Arc<RwLock<SongInternalLevelIndex>>,
    pub(crate) song_data_root: Arc<RwLock<Vec<SongCatalogSong>>>,
    pub(crate) song_data_base_path: PathBuf,
    pub(crate) song_data_loaded: Arc<AtomicBool>,
}

impl AppState {
    pub(crate) fn reload_song_data(&self) -> eyre::Result<()> {
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

pub(crate) fn load_song_data(
    path: &Path,
) -> eyre::Result<(SongCatalog, SongInternalLevelIndex, bool)> {
    if !path.exists() {
        return Ok((
            SongCatalog { songs: Vec::new() },
            SongInternalLevelIndex::empty(),
            false,
        ));
    }

    let bytes = std::fs::read(path).wrap_err("read song data")?;
    let root: SongCatalog = serde_json::from_slice(&bytes).wrap_err("parse song data")?;
    let index_root: SongCatalog =
        serde_json::from_slice(&bytes).wrap_err("parse song data for index")?;
    let index = SongInternalLevelIndex::from_catalog(index_root);

    Ok((root, index, true))
}
