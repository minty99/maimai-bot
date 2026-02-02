use clap::Parser;
use eyre::WrapErr;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "maimai-songdb")]
#[command(about = "Standalone maimai song database builder (non-CN)", long_about = None)]
struct Args {
    /// Target directory to output data.json, images
    #[arg(short, long, default_value = "data")]
    target: PathBuf,
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
    let database = maimai_songdb::SongDatabase::fetch(&config, &args.target)
        .await
        .wrap_err("failed to fetch song database")?;

    tracing::info!("Writing data.json...");
    let data_root = database
        .into_data_root()
        .wrap_err("failed to convert to data root")?;
    let json_bytes = serde_json::to_vec_pretty(&data_root).wrap_err("serialize data.json")?;
    std::fs::write(args.target.join("data.json"), json_bytes).wrap_err("write data.json")?;

    tracing::info!("✓ Successfully updated maimai song database");
    tracing::info!("  - JSON: {}/data.json", args.target.display());
    tracing::info!("  - Covers: {}/cover/", args.target.display());

    Ok(())
}
