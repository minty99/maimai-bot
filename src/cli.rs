use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use reqwest::Url;

#[derive(Debug, Parser)]
#[command(name = "maimai-bot")]
#[command(about = "maimai DX NET Discord bot (single user)")]
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
    pub command: Option<Command>,
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

    #[command(
        about = "Run a single local command to simulate Discord slash commands (no Discord usage)"
    )]
    Simulate {
        #[arg(
            long,
            default_value = "pretty",
            value_enum,
            help = "Output format for the generated CreateReply payload"
        )]
        format: SimulateFormat,

        #[arg(
            long,
            default_value = "maimai-user",
            value_name = "NAME",
            help = "Display name used for embed titles"
        )]
        display_name: String,

        #[arg(
            value_name = "CMD",
            trailing_var_arg = true,
            help = "Run a single simulate command and exit (e.g., mai-score \"Song Title\")"
        )]
        command: Vec<String>,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SimulateFormat {
    Pretty,
    Json,
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
    #[command(about = "Crawl playerData page and write parsed JSON")]
    PlayerData {
        #[arg(
            long,
            default_value = "data/out/player_data.json",
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
    #[command(
        about = "Rebuild DB from scratch (clear tables, sync scores + recent, store player snapshot)"
    )]
    Build,
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
