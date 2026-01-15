use std::path::PathBuf;

use clap::{Parser, Subcommand};
use reqwest::Url;

#[derive(Debug, Parser)]
#[command(name = "maimai-bot")]
#[command(about = "maimai DX NET crawler (single user)")]
pub struct RootArgs {
    #[arg(long, default_value = "data")]
    pub data_dir: PathBuf,

    #[arg(long, default_value = "data/cookies.json")]
    pub cookie_path: PathBuf,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
    },
    Fetch {
        #[command(subcommand)]
        command: FetchCommand,
    },
    Crawl {
        #[command(subcommand)]
        command: CrawlCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum AuthCommand {
    Check,
    Login,
}

#[derive(Debug, Subcommand)]
pub enum FetchCommand {
    Url {
        #[arg(long)]
        url: Url,
        #[arg(long)]
        out: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
pub enum CrawlCommand {
    Scores {
        #[arg(long)]
        diff: Option<u8>,

        #[arg(long, default_value = "data/out/scores.json")]
        out: PathBuf,
    },
}
