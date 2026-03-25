use std::path::Path;

use chrono::Utc;
use eyre::WrapErr;

use crate::songdb::{SongDatabase, SongDbConfig};

pub(crate) async fn generate_song_database(song_data_base_path: &Path) -> eyre::Result<()> {
    tracing::info!("songdb: starting generation");

    let config =
        SongDbConfig::from_env().wrap_err("songdb env not configured; cannot generate data")?;

    std::fs::create_dir_all(song_data_base_path).wrap_err("create song_data output dir")?;

    let database = SongDatabase::fetch(&config, song_data_base_path)
        .await
        .wrap_err("failed to fetch song database")?;

    let catalog = database
        .into_data_root()
        .wrap_err("failed to convert to data root")?;
    let data_root = build_song_database_root(catalog);

    let json_bytes = serde_json::to_vec_pretty(&data_root).wrap_err("serialize data.json")?;
    std::fs::write(song_data_base_path.join("data.json"), json_bytes)
        .wrap_err("write data.json")?;

    Ok(())
}

fn build_song_database_root(catalog: models::SongCatalog) -> models::SongDatabase {
    models::SongDatabase {
        generated_at: Utc::now().to_rfc3339(),
        songs: catalog.songs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_song_database_root_sets_generated_at_and_preserves_songs() {
        let catalog = models::SongCatalog { songs: Vec::new() };
        let root = build_song_database_root(catalog);

        assert!(chrono::DateTime::parse_from_rfc3339(&root.generated_at).is_ok());
        assert!(root.songs.is_empty());
    }
}
