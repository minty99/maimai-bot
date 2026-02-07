use eyre::WrapErr;

#[derive(Debug, Clone)]
pub(crate) struct RecordCollectorConfig {
    pub(crate) sega_id: String,
    pub(crate) sega_password: String,
    pub(crate) port: u16,
    pub(crate) database_url: String,
    pub(crate) data_dir: String,
    pub(crate) song_info_server_url: String,
}

impl RecordCollectorConfig {
    pub(crate) fn from_env() -> eyre::Result<Self> {
        let sega_id = std::env::var("SEGA_ID").wrap_err("missing env var: SEGA_ID")?;
        let sega_password =
            std::env::var("SEGA_PASSWORD").wrap_err("missing env var: SEGA_PASSWORD")?;
        let port = std::env::var("RECORD_COLLECTOR_PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse::<u16>()
            .wrap_err("RECORD_COLLECTOR_PORT must be a valid u16")?;
        let database_url =
            std::env::var("DATABASE_URL").wrap_err("missing env var: DATABASE_URL")?;
        let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string());
        let song_info_server_url = std::env::var("SONG_INFO_SERVER_URL")
            .unwrap_or_else(|_| "http://localhost:3001".to_string());

        Ok(Self {
            sega_id,
            sega_password,
            port,
            database_url,
            data_dir,
            song_info_server_url,
        })
    }
}
