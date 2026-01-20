use std::path::PathBuf;

use clap::{Parser, Subcommand};
use reqwest::Url;

#[derive(Debug, Parser)]
#[command(name = "maimai-bot")]
#[command(about = "maimai DX NET crawler (single user)")]
#[command(arg_required_else_help = true)]
pub struct RootArgs {
    #[arg(
        long,
        default_value = "data",
        value_name = "DIR",
        help = "Directory for local runtime data (cookies, sqlite, debug HTML)"
    )]
    pub data_dir: PathBuf,

    #[arg(
        long,
        default_value = "data/cookies.json",
        value_name = "FILE",
        help = "Path to persisted cookie store JSON"
    )]
    pub cookie_path: PathBuf,

    #[arg(
        long,
        default_value = "data/maimai.sqlite3",
        value_name = "FILE",
        help = "Path to SQLite database file"
    )]
    pub db_path: PathBuf,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(about = "Authenticate and manage session cookies")]
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
    },
    #[command(about = "Fetch a page and save the raw HTML")]
    Fetch {
        #[command(subcommand)]
        command: FetchCommand,
    },
    #[command(about = "Crawl pages and output parsed JSON (no DB)")]
    Crawl {
        #[command(subcommand)]
        command: CrawlCommand,
    },
    #[command(about = "Initialize and sync data into the SQLite database")]
    Db {
        #[command(subcommand)]
        command: DbCommand,
    },
    #[command(about = "Run Discord bot")]
    Bot {
        #[command(subcommand)]
        command: BotCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum AuthCommand {
    #[command(about = "Check whether the current cookie session is authenticated")]
    Check,
    #[command(about = "Log in using SEGA_ID/SEGA_PASSWORD and persist cookies")]
    Login,
}

#[derive(Debug, Subcommand)]
pub enum FetchCommand {
    #[command(about = "Fetch a URL with authentication and save response body to a file")]
    Url {
        #[arg(long, value_name = "URL", help = "Target URL to fetch")]
        url: Url,
        #[arg(
            long,
            value_name = "FILE",
            help = "Output file path to write the response body"
        )]
        out: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
pub enum CrawlCommand {
    #[command(about = "Crawl song scores list page(s) and write parsed JSON")]
    Scores {
        #[arg(
            long,
            value_name = "0..4",
            help = "Difficulty (0 BASIC, 1 ADVANCED, 2 EXPERT, 3 MASTER, 4 Re:MASTER). Omit for all."
        )]
        diff: Option<u8>,

        #[arg(
            long,
            default_value = "data/out/scores.json",
            value_name = "FILE",
            help = "Output JSON file path"
        )]
        out: PathBuf,
    },
    #[command(about = "Crawl recent play records (latest 50) and write parsed JSON")]
    Recent {
        #[arg(
            long,
            default_value = "data/out/recent.json",
            value_name = "FILE",
            help = "Output JSON file path"
        )]
        out: PathBuf,
    },
    #[command(about = "Crawl a song detail page (musicDetail) and write parsed JSON")]
    SongDetail {
        #[arg(
            long,
            value_name = "IDX",
            help = "musicDetail idx parameter value (use the idx value from scores list)"
        )]
        idx: String,

        #[arg(
            long,
            default_value = "data/out/song_detail.json",
            value_name = "FILE",
            help = "Output JSON file path"
        )]
        out: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
pub enum DbCommand {
    #[command(about = "Create DB file and run migrations")]
    Init,
    #[command(about = "Fetch scores from maimai DX NET and upsert into DB")]
    SyncScores {
        #[arg(
            long,
            value_name = "0..4",
            help = "Difficulty (0 BASIC, 1 ADVANCED, 2 EXPERT, 3 MASTER, 4 Re:MASTER). Omit for all."
        )]
        diff: Option<u8>,
    },
    #[command(about = "Fetch recent play records and upsert into DB")]
    SyncRecent,
}

#[derive(Debug, Subcommand)]
pub enum BotCommand {
    #[command(about = "Start Discord bot")]
    Run,
}
