use clap::Parser;
use eyre::WrapErr;

use maimai_bot::cli::{
    AuthCommand, BotCommand, Command, CrawlCommand, DbCommand, FetchCommand, RootArgs,
};
use maimai_bot::config::AppConfig;
use maimai_bot::db;
use maimai_bot::http::MaimaiClient;
use maimai_bot::maimai::parse::player_data::parse_player_data_html;
use maimai_bot::maimai::parse::recent::parse_recent_html;
use maimai_bot::maimai::parse::score_list::parse_scores_html;
use maimai_bot::maimai::parse::song_detail::parse_song_detail_html;
use reqwest::Url;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    dotenvy::dotenv().ok();
    init_tracing();

    let args = RootArgs::parse();
    let config = AppConfig::from_env_and_args(&args).wrap_err("load config")?;
    config.ensure_dirs().wrap_err("create data directories")?;

    let mut client = MaimaiClient::new(&config).wrap_err("initialize http client")?;

    let command = args.command.unwrap_or(Command::Bot {
        command: BotCommand::Run,
    });

    match command {
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
        Command::Crawl {
            command: CrawlCommand::Recent { out },
        } => {
            client
                .ensure_logged_in()
                .await
                .wrap_err("ensure logged in")?;
            let url = record_url()?;
            let bytes = client.get_bytes(&url).await.wrap_err("fetch record url")?;
            let html = String::from_utf8(bytes).wrap_err("record response is not utf-8")?;
            let entries = parse_recent_html(&html).wrap_err("parse recent html")?;

            std::fs::create_dir_all(
                out.parent()
                    .ok_or_else(|| eyre::eyre!("invalid --out path: {out:?}"))?,
            )
            .wrap_err("create output directory")?;
            let json = serde_json::to_string_pretty(&entries).wrap_err("serialize json")?;
            std::fs::write(&out, json).wrap_err("write json")?;
            println!("saved={}", out.display());
        }
        Command::Crawl {
            command: CrawlCommand::SongDetail { idx, out },
        } => {
            client
                .ensure_logged_in()
                .await
                .wrap_err("ensure logged in")?;
            let url = song_detail_url(&idx)?;
            let bytes = client
                .get_bytes(&url)
                .await
                .wrap_err("fetch song detail url")?;
            let html = String::from_utf8(bytes).wrap_err("song detail response is not utf-8")?;
            let parsed = parse_song_detail_html(&html).wrap_err("parse song detail html")?;

            std::fs::create_dir_all(
                out.parent()
                    .ok_or_else(|| eyre::eyre!("invalid --out path: {out:?}"))?,
            )
            .wrap_err("create output directory")?;
            let json = serde_json::to_string_pretty(&parsed).wrap_err("serialize json")?;
            std::fs::write(&out, json).wrap_err("write json")?;
            println!("saved={}", out.display());
        }
        Command::Crawl {
            command: CrawlCommand::PlayerData { out },
        } => {
            client
                .ensure_logged_in()
                .await
                .wrap_err("ensure logged in")?;
            let url = player_data_url()?;
            let bytes = client
                .get_bytes(&url)
                .await
                .wrap_err("fetch playerData url")?;
            let html = String::from_utf8(bytes).wrap_err("playerData response is not utf-8")?;
            let parsed = parse_player_data_html(&html).wrap_err("parse playerData html")?;

            std::fs::create_dir_all(
                out.parent()
                    .ok_or_else(|| eyre::eyre!("invalid --out path: {out:?}"))?,
            )
            .wrap_err("create output directory")?;
            let json = serde_json::to_string_pretty(&parsed).wrap_err("serialize json")?;
            std::fs::write(&out, json).wrap_err("write json")?;
            println!("saved={}", out.display());
        }
        Command::Db { command } => match command {
            DbCommand::Init => {
                ensure_parent_dir(&args.db_path).wrap_err("create db parent directory")?;
                let pool = db::connect(&args.db_path).await.wrap_err("connect db")?;
                db::migrate(&pool).await.wrap_err("migrate db")?;
                println!("db_init=ok path={}", args.db_path.display());
            }
            DbCommand::SyncScores { diff } => {
                client
                    .ensure_logged_in()
                    .await
                    .wrap_err("ensure logged in")?;
                ensure_parent_dir(&args.db_path).wrap_err("create db parent directory")?;
                let pool = db::connect(&args.db_path).await.wrap_err("connect db")?;
                db::migrate(&pool).await.wrap_err("migrate db")?;

                let diffs: Vec<u8> = match diff {
                    Some(d) => vec![d],
                    None => vec![0, 1, 2, 3, 4],
                };

                let scraped_at = unix_timestamp();
                let mut all = Vec::new();
                for d in diffs {
                    let url = scores_url(d)?;
                    let bytes = client.get_bytes(&url).await.wrap_err("fetch scores url")?;
                    let html = String::from_utf8(bytes).wrap_err("scores response is not utf-8")?;
                    let mut entries = parse_scores_html(&html, d).wrap_err("parse scores html")?;
                    all.append(&mut entries);
                }

                let count = all.len();
                db::upsert_scores(&pool, scraped_at, &all)
                    .await
                    .wrap_err("upsert scores")?;
                println!("db_sync_scores=ok entries={count}");
            }
            DbCommand::SyncRecent => {
                client
                    .ensure_logged_in()
                    .await
                    .wrap_err("ensure logged in")?;
                ensure_parent_dir(&args.db_path).wrap_err("create db parent directory")?;
                let pool = db::connect(&args.db_path).await.wrap_err("connect db")?;
                db::migrate(&pool).await.wrap_err("migrate db")?;

                let url = Url::parse("https://maimaidx-eng.com/maimai-mobile/record/")
                    .wrap_err("parse record url")?;
                let bytes = client.get_bytes(&url).await.wrap_err("fetch record url")?;
                let html = String::from_utf8(bytes).wrap_err("record response is not utf-8")?;
                let entries = parse_recent_html(&html).wrap_err("parse recent html")?;

                let scraped_at = unix_timestamp();
                let count_total = entries.len();
                let count_with_idx = entries
                    .iter()
                    .filter(|e| e.played_at_unixtime.is_some())
                    .count();
                db::upsert_playlogs(&pool, scraped_at, &entries)
                    .await
                    .wrap_err("upsert playlogs")?;
                println!(
                    "db_sync_recent=ok entries_total={count_total} entries_with_idx={count_with_idx}"
                );
            }
        },
        Command::Bot { command } => match command {
            BotCommand::Run => {
                ensure_parent_dir(&args.db_path).wrap_err("create db parent directory")?;
                maimai_bot::discord::run_bot(config, args.db_path.clone())
                    .await
                    .wrap_err("run discord bot")?;
            }
        },
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

fn unix_timestamp() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn ensure_parent_dir(path: &std::path::Path) -> eyre::Result<()> {
    let Some(parent) = path.parent() else {
        return Err(eyre::eyre!("invalid path: {path:?}"));
    };
    if parent.as_os_str().is_empty() {
        return Ok(());
    }
    std::fs::create_dir_all(parent).wrap_err("create parent dir")?;
    Ok(())
}

fn record_url() -> eyre::Result<Url> {
    Url::parse("https://maimaidx-eng.com/maimai-mobile/record/").wrap_err("parse record url")
}

fn song_detail_url(idx: &str) -> eyre::Result<Url> {
    let mut url = Url::parse("https://maimaidx-eng.com/maimai-mobile/record/musicDetail/")
        .wrap_err("parse song detail base url")?;
    url.query_pairs_mut().append_pair("idx", idx);
    Ok(url)
}

fn player_data_url() -> eyre::Result<Url> {
    Url::parse("https://maimaidx-eng.com/maimai-mobile/playerData/")
        .wrap_err("parse playerData url")
}
