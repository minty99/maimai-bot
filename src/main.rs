use clap::Parser;
use eyre::WrapErr;

use maimai_bot::cli::{AuthCommand, Command, CrawlCommand, FetchCommand, RootArgs};
use maimai_bot::config::AppConfig;
use maimai_bot::http::MaimaiClient;
use maimai_bot::maimai::parse::score_list::parse_scores_html;
use reqwest::Url;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    dotenvy::dotenv().ok();
    init_tracing();

    let args = RootArgs::parse();
    let config = AppConfig::from_env_and_args(&args).wrap_err("load config")?;
    config.ensure_dirs().wrap_err("create data directories")?;

    let mut client = MaimaiClient::new(&config).wrap_err("initialize http client")?;

    match args.command {
        Command::Auth {
            command: AuthCommand::Check,
        } => {
            let logged_in = client.check_logged_in().await.wrap_err("auth check")?;
            if logged_in {
                println!("logged_in=true");
            } else {
                println!("logged_in=false");
            }
        }
        Command::Auth {
            command: AuthCommand::Login,
        } => {
            client.login().await.wrap_err("login")?;
            println!("login=ok");
        }
        Command::Fetch {
            command: FetchCommand::Url { url, out },
        } => {
            client
                .ensure_logged_in()
                .await
                .wrap_err("ensure logged in")?;
            let bytes = client.get_bytes(&url).await.wrap_err("fetch url")?;
            std::fs::create_dir_all(
                out.parent()
                    .ok_or_else(|| eyre::eyre!("invalid --out path: {out:?}"))?,
            )
            .wrap_err("create output directory")?;
            std::fs::write(&out, &bytes).wrap_err("write output file")?;
            println!("saved={}", out.display());
        }
        Command::Crawl {
            command: CrawlCommand::Scores { diff, out },
        } => {
            client
                .ensure_logged_in()
                .await
                .wrap_err("ensure logged in")?;

            let diffs: Vec<u8> = match diff {
                Some(d) => vec![d],
                None => vec![0, 1, 2, 3, 4],
            };

            let mut all = Vec::new();
            for d in diffs {
                let url = scores_url(d)?;
                let bytes = client.get_bytes(&url).await.wrap_err("fetch scores url")?;
                let html = String::from_utf8(bytes).wrap_err("scores response is not utf-8")?;
                let mut entries = parse_scores_html(&html, d).wrap_err("parse scores html")?;
                all.append(&mut entries);
            }

            std::fs::create_dir_all(
                out.parent()
                    .ok_or_else(|| eyre::eyre!("invalid --out path: {out:?}"))?,
            )
            .wrap_err("create output directory")?;

            let json = serde_json::to_string_pretty(&all).wrap_err("serialize json")?;
            std::fs::write(&out, json).wrap_err("write json")?;
            println!("saved={}", out.display());
        }
    }

    Ok(())
}

fn init_tracing() {
    let env_filter =
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());
    tracing_subscriber::fmt().with_env_filter(env_filter).init();
}

fn scores_url(diff: u8) -> eyre::Result<Url> {
    if diff > 4 {
        return Err(eyre::eyre!("diff must be 0..4"));
    }
    Url::parse(&format!(
        "https://maimaidx-eng.com/maimai-mobile/record/musicGenre/search/?genre=99&diff={diff}"
    ))
    .wrap_err("parse scores url")
}
