#[derive(Debug, Clone)]
pub(crate) struct Config {
    pub(crate) song_data_path: String,
}

impl Config {
    pub(crate) fn from_env() -> eyre::Result<Self> {
        let song_data_path =
            std::env::var("SONG_DATA_PATH").unwrap_or_else(|_| "data/song_data".to_string());

        Ok(Self { song_data_path })
    }
}
