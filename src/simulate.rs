use std::path::PathBuf;

use eyre::{Result, WrapErr};
use poise::serenity_prelude as serenity;
use serde::Serialize;
use serenity::builder::CreateEmbed;

use crate::cli::SimulateFormat;
use crate::db::{self, SqlitePool};
use crate::discord::bot::{
    embed_base, normalize_for_match, sync_from_network_without_discord, top_title_matches,
};
use crate::discord::mai_commands;
use crate::http::MaimaiClient;
use crate::song_data::SongDataIndex;

#[derive(Debug, Clone, Serialize)]
pub struct ReplyPayload {
    pub content: Option<String>,
    pub embeds: Vec<CreateEmbed>,
}

impl ReplyPayload {
    fn embed(embed: CreateEmbed) -> Self {
        Self {
            content: None,
            embeds: vec![embed],
        }
    }
}

#[derive(Debug, Clone)]
pub struct SimulateArgs {
    pub format: SimulateFormat,
    pub display_name: String,
    pub command: Vec<String>,
}

pub async fn run_simulate(
    db_path: PathBuf,
    client: &mut MaimaiClient,
    args: SimulateArgs,
) -> Result<()> {
    let pool = db::connect(&db_path).await.wrap_err("connect db")?;
    db::migrate(&pool).await.wrap_err("migrate db")?;

    let song_data = match SongDataIndex::load_from_default_locations() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("warn: failed to load song data (non-fatal): {e:?}");
            None
        }
    };

    if args.command.is_empty() {
        let reply = ReplyPayload::embed(
            embed_base("Invalid arguments").description(
                "Usage: simulate <cmd> [args...]\n\nExamples:\n- simulate mai-score \"Song Title\"\n- simulate mai-recent\n- simulate mai-rating\n- simulate mai-today",
            ),
        );
        print_reply(&reply, args.format)?;
        return Ok(());
    }

    let reply = execute_simulate_command(&pool, song_data.as_ref(), client, &args).await;
    print_reply(&reply, args.format)?;
    Ok(())
}

async fn execute_simulate_command(
    pool: &SqlitePool,
    song_data: Option<&SongDataIndex>,
    client: &mut MaimaiClient,
    args: &SimulateArgs,
) -> ReplyPayload {
    let cmd_raw = args.command.first().map(String::as_str).unwrap_or("");
    let cmd = cmd_raw.trim();
    let rest = args.command.get(1..).unwrap_or_default().join(" ");
    let rest = rest.trim();

    let should_sync = matches!(
        cmd,
        "/mai-score"
            | "mai-score"
            | "/mai-recent"
            | "mai-recent"
            | "/mai-rating"
            | "mai-rating"
            | "/mai-today"
            | "mai-today"
    );

    let display_name = if should_sync {
        match sync_from_network_without_discord(pool, client).await {
            Ok(player) => player.user_name,
            Err(e) => {
                eprintln!("warn: network sync failed: {e:#}");
                args.display_name.clone()
            }
        }
    } else {
        args.display_name.clone()
    };

    match cmd {
        "/mai-score" | "mai-score" => {
            if rest.is_empty() {
                return ReplyPayload::embed(
                    embed_base("Invalid arguments").description("Usage: /mai-score <title>"),
                );
            }

            let titles = match mai_commands::fetch_score_titles(pool).await {
                Ok(v) => v,
                Err(e) => {
                    return ReplyPayload::embed(embed_base("Error").description(format!("{e:#}")));
                }
            };

            if titles.is_empty() {
                return ReplyPayload::embed(mai_commands::embed_no_scores_found());
            }

            let search_norm = normalize_for_match(rest);
            let exact_title = titles
                .iter()
                .find(|t| normalize_for_match(t) == search_norm)
                .cloned();

            let matched_title = if let Some(exact) = exact_title {
                exact
            } else {
                let candidates = top_title_matches(rest, &titles, 5);
                if candidates.is_empty() {
                    return ReplyPayload::embed(
                        embed_base("No records found").description("No titles to match."),
                    );
                }

                let mut lines = Vec::new();
                for (i, title) in candidates.iter().enumerate() {
                    lines.push(format!("`{}` {}", i + 1, title));
                }

                return ReplyPayload::embed(embed_base("No exact match").description(format!(
                    "Query: `{rest}`\n\n{}\n\nRe-run with one of the titles above.",
                    lines.join("\n")
                )));
            };

            match mai_commands::build_mai_score_embed_for_title(
                pool,
                song_data,
                &display_name,
                &matched_title,
            )
            .await
            {
                Ok((embed, _has_rows)) => ReplyPayload::embed(embed),
                Err(e) => ReplyPayload::embed(embed_base("Error").description(format!("{e:#}"))),
            }
        }

        "/mai-recent" | "mai-recent" => {
            match mai_commands::build_mai_recent_embeds_for_latest_credit(
                pool,
                song_data,
                &display_name,
            )
            .await
            {
                Ok(embeds) => ReplyPayload {
                    content: None,
                    embeds,
                },
                Err(e) => ReplyPayload::embed(embed_base("Error").description(format!("{e:#}"))),
            }
        }

        "/mai-rating" | "mai-rating" => {
            match mai_commands::build_mai_rating_embeds(pool, song_data, &display_name).await {
                Ok(embeds) => ReplyPayload {
                    content: None,
                    embeds,
                },
                Err(e) => ReplyPayload::embed(embed_base("Error").description(format!("{e:#}"))),
            }
        }

        "/mai-today" | "mai-today" => {
            match mai_commands::build_mai_today_embed_for_now(pool, &display_name).await {
                Ok(embed) => ReplyPayload::embed(embed),
                Err(e) => ReplyPayload::embed(embed_base("Error").description(format!("{e:#}"))),
            }
        }

        _ => ReplyPayload::embed(
            embed_base("Unknown command").description("Try `--help` for available commands."),
        ),
    }
}

fn print_reply(reply: &ReplyPayload, format: SimulateFormat) -> Result<()> {
    match format {
        SimulateFormat::Json => {
            let json = serde_json::to_string_pretty(reply).wrap_err("serialize reply payload")?;
            println!("{json}");
        }
        SimulateFormat::Pretty => {
            let v = serde_json::to_value(reply).wrap_err("serialize reply payload")?;

            if let Some(content) = v.get("content").and_then(|c| c.as_str())
                && !content.trim().is_empty()
            {
                println!("Content:\n{content}\n");
            }

            let embeds = v
                .get("embeds")
                .and_then(|e| e.as_array())
                .cloned()
                .unwrap_or_default();
            if embeds.is_empty() {
                println!("(no embeds)");
                return Ok(());
            }

            for (i, e) in embeds.iter().enumerate() {
                let title = e.get("title").and_then(|x| x.as_str()).unwrap_or("");
                let desc = e.get("description").and_then(|x| x.as_str()).unwrap_or("");
                let thumb = e
                    .get("thumbnail")
                    .and_then(|t| t.get("url"))
                    .and_then(|x| x.as_str());

                if embeds.len() > 1 {
                    println!("Embed {}:", i + 1);
                }
                if !title.is_empty() {
                    println!("Title: {title}");
                }
                if !desc.is_empty() {
                    println!("Description:\n{desc}");
                }
                if let Some(url) = thumb {
                    println!("Thumbnail: {url}");
                }

                let fields = e
                    .get("fields")
                    .and_then(|f| f.as_array())
                    .cloned()
                    .unwrap_or_default();
                if !fields.is_empty() {
                    println!("Fields:");
                    for f in fields {
                        let name = f.get("name").and_then(|x| x.as_str()).unwrap_or("");
                        let value = f.get("value").and_then(|x| x.as_str()).unwrap_or("");
                        if !name.is_empty() || !value.is_empty() {
                            println!("- {name}: {value}");
                        }
                    }
                }

                if i + 1 != embeds.len() {
                    println!();
                }
            }
        }
    }
    Ok(())
}
