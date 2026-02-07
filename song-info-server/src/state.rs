use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use models::{SongDataIndex, SongDataSong};

#[derive(Clone)]
pub struct AppState {
    pub song_data: Arc<RwLock<SongDataIndex>>,
    pub song_data_root: Arc<Vec<SongDataSong>>,
    pub song_data_base_path: PathBuf,
    pub song_data_loaded: bool,
}
