use eyre::WrapErr;

#[derive(Debug, Clone)]
pub(crate) struct Config {
    pub(crate) port: u16,
    pub(crate) song_data_path: String,
}

impl Config {
    pub(crate) fn from_env() -> eyre::Result<Self> {
        let port = std::env::var("SONG_INFO_PORT")
            .unwrap_or_else(|_| "3001".to_string())
            .parse::<u16>()
            .wrap_err("SONG_INFO_PORT must be a valid u16")?;
        let song_data_path =
            std::env::var("SONG_DATA_PATH").unwrap_or_else(|_| "data/song_data".to_string());

        Ok(Self {
            port,
            song_data_path,
        })
    }
}
