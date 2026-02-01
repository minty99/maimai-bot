use clap::Parser;
use eyre::WrapErr;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "maimai-songdb")]
#[command(about = "Standalone maimai song database builder (non-CN)", long_about = None)]
struct Args {
    /// Target directory to output data.json, images
    #[arg(short, long, default_value = "data")]
    target: PathBuf,
}

#[derive(Debug, Serialize)]
struct SongDataRoot {
    songs: Vec<SongDataSong>,
}

#[derive(Debug, Serialize)]
struct SongDataSong {
    title: String,
    version: Option<String>,
    #[serde(rename = "imageName")]
    image_name: Option<String>,
    sheets: Vec<SongDataSheet>,
}

#[derive(Debug, Serialize)]
struct SongDataSheet {
    #[serde(rename = "type")]
    sheet_type: String,
    difficulty: String,
    #[serde(rename = "internalLevelValue")]
    internal_level_value: f32,
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "maimai_songdb=info".into()),
        )
        .init();

    let args = Args::parse();

    std::fs::create_dir_all(&args.target)
        .wrap_err_with(|| format!("failed to create target directory: {:?}", args.target))?;

    let config =
        maimai_songdb::SongDbConfig::from_env().wrap_err("failed to load config from env vars")?;

    tracing::info!("Target directory: {:?}", args.target);
    tracing::info!("Config loaded: {:?}", config);

    tracing::info!("Starting maimai songdb update...");
    let data = maimai_songdb::SongDatabase::fetch(&config, &args.target)
        .await
        .wrap_err("failed to fetch song database")?;

    tracing::info!("Writing data.json...");
    let json_output = build_json_output(&data)?;
    let json_bytes = serde_json::to_vec_pretty(&json_output).wrap_err("serialize data.json")?;
    std::fs::write(args.target.join("data.json"), json_bytes).wrap_err("write data.json")?;

    tracing::info!("✓ Successfully updated maimai song database");
    tracing::info!("  - JSON: {}/data.json", args.target.display());
    tracing::info!("  - Covers: {}/cover/", args.target.display());

    Ok(())
}

fn build_json_output(data: &maimai_songdb::SongDatabase) -> eyre::Result<SongDataRoot> {
    use std::collections::BTreeMap;

    let mut song_map: BTreeMap<String, SongDataSong> = BTreeMap::new();

    for song in &data.songs {
        song_map.insert(
            song.song_id.clone(),
            SongDataSong {
                title: song.title.clone(),
                version: song.version.clone(),
                image_name: Some(song.image_name.clone()),
                sheets: Vec::new(),
            },
        );
    }

    for sheet in &data.sheets {
        let song = match song_map.get_mut(&sheet.song_id) {
            Some(song) => song,
            None => continue,
        };

        let key = (
            sheet.song_id.clone(),
            sheet.sheet_type.clone(),
            sheet.difficulty.clone(),
        );
        let internal_level = data.internal_levels.get(&key);

        let Some(internal_level_str) = internal_level.map(|il| &il.internal_level) else {
            continue;
        };

        let internal_level_value = internal_level_str
            .trim()
            .parse::<f32>()
            .wrap_err("parse internal_level as f32")?;

        song.sheets.push(SongDataSheet {
            sheet_type: sheet.sheet_type.clone(),
            difficulty: sheet.difficulty.clone(),
            internal_level_value,
        });
    }

    Ok(SongDataRoot {
        songs: song_map.into_values().collect(),
    })
}
