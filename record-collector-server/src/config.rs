use eyre::{WrapErr, eyre};

use crate::backup::url::parse_s3_url;

#[derive(Debug, Clone)]
pub(crate) struct BackupConfig {
    pub(crate) s3_bucket: String,
    pub(crate) s3_prefix: String,
    pub(crate) region: String,
}

#[derive(Debug, Clone)]
pub(crate) struct RecordCollectorConfig {
    pub(crate) sega_id: String,
    pub(crate) sega_password: String,
    pub(crate) port: u16,
    pub(crate) database_url: String,
    pub(crate) data_dir: String,
    pub(crate) backup: Option<BackupConfig>,
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
        let backup = load_backup_config_from_env()?;

        Ok(Self {
            sega_id,
            sega_password,
            port,
            database_url,
            data_dir,
            backup,
        })
    }
}

fn load_backup_config_from_env() -> eyre::Result<Option<BackupConfig>> {
    let raw_url = match std::env::var("BACKUP_S3_URL") {
        Ok(value) => value.trim().to_string(),
        Err(std::env::VarError::NotPresent) => return Ok(None),
        Err(err) => return Err(eyre!("read BACKUP_S3_URL: {err}")),
    };
    if raw_url.is_empty() {
        return Ok(None);
    }

    let parsed = parse_s3_url(&raw_url).wrap_err("parse BACKUP_S3_URL")?;
    let region = std::env::var("BACKUP_S3_REGION").wrap_err("missing env var: BACKUP_S3_REGION")?;
    let region = region.trim().to_string();
    if region.is_empty() {
        return Err(eyre!("BACKUP_S3_REGION must not be empty"));
    }

    Ok(Some(BackupConfig {
        s3_bucket: parsed.bucket,
        s3_prefix: parsed.prefix,
        region,
    }))
}
