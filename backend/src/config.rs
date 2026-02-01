use eyre::WrapErr;

#[derive(Debug, Clone)]
pub struct BackendConfig {
    pub sega_id: String,
    pub sega_password: String,
    pub port: u16,
    pub database_url: String,
    pub data_dir: String,
}

impl BackendConfig {
    pub fn from_env() -> eyre::Result<Self> {
        let sega_id = std::env::var("SEGA_ID").wrap_err("missing env var: SEGA_ID")?;
        let sega_password =
            std::env::var("SEGA_PASSWORD").wrap_err("missing env var: SEGA_PASSWORD")?;
        let port = std::env::var("BACKEND_PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse::<u16>()
            .wrap_err("BACKEND_PORT must be a valid u16")?;
        let database_url =
            std::env::var("DATABASE_URL").wrap_err("missing env var: DATABASE_URL")?;
        let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string());

        Ok(Self {
            sega_id,
            sega_password,
            port,
            database_url,
            data_dir,
        })
    }
}
