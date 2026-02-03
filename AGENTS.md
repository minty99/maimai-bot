# Agent Guidelines (maimai-bot)

This repo is a **single-user** Rust app that logs in to **maimaidx-eng.com** (SEGA ID), crawls your data, stores it locally (SQLite), and exposes it via a Discord bot.

The goal of this file is to keep future changes consistent with the current implementation and constraints.

## Non-negotiables

- **Single user only**: no multi-user concepts (accounts table, user scoping, per-user DB rows, etc.).
- **Secrets never committed**: `.env` and `data/` are always gitignored; never print credentials or cookie contents.
- **Dependency management**: use `cargo add` / `cargo remove` (avoid hand-editing `Cargo.toml`).
- **Error handling**: use `eyre` (`eyre::Result`, `WrapErr`) consistently.
- **Validation**: prefer `cargo fmt`, `cargo clippy`, `cargo test`.

## Runtime architecture (current)

- CLI entrypoint: `src/main.rs` (clap). Key subcommands:
  - `auth check|login`
  - `fetch url --url <URL> --out <FILE>` (authenticated fetch, raw bytes)
  - `crawl scores|recent|song-detail|player-data` (parse to JSON; no DB)
  - `db init|sync-scores|sync-recent`
  - `bot run` (Discord bot)
- Discord bot: `src/discord/bot.rs`
  - On startup:
    - fetches `playerData` and stores `player.user_name` in memory (`BotData.maimai_user_name`)
    - checks if scores sync is needed using `app_state`'s `player.total_play_count`
    - if needed: sync scores (diff 0..4) + seed recent playlogs once
    - sends a startup DM embed with player data
  - Background loop (every 10 minutes):
    - fetches `playerData` again
    - only if **total play count changed**: fetches recent page, upserts playlogs, then sends a DM embed for "New plays detected"
  - Slash commands:
    - `/mai-score <title>`: selects a single best match
      - exact match: show that title
      - non-exact: show top 5 candidates as buttons; on click delete the prompt message, then display the selected title's scores
      - hide "unplayed" rows (`achievement_x10000 IS NULL`)
    - `/mai-recent`: shows the latest credit (based on recent page's TRACK numbering), formatted from `TRACK 01` upwards

## DB & migrations

- Migrations under `migrations/` are executed at runtime via `sqlx::migrate!()` (`src/db/mod.rs`).
- Core tables:
  - `scores` primary key: `(title, chart_type, diff_category)`
  - `playlogs` primary key: `playlog_idx`
  - `app_state` key/value store for small snapshots (e.g. total play count, rating)
- `achievement_x10000` is stored as integer (`percent * 10000`, rounded). Read paths typically divide by `10000.0` and format with 4 decimals.

## Model conventions (important for future edits)

- Enums exist for stable UI/DB strings:
  - `ChartType` (`STD|DX`)
  - `DifficultyCategory` (`BASIC..Re:MASTER`)
  - `ScoreRank` (`SSS+ .. D`)
  - `FcStatus` (`FC/FC+/AP/AP+`)
  - `SyncStatus` (`SYNC/FS/FS+/FDX/FDX+`)
- Parsers should return enums, and DB bindings should store `as_str()` results.

## Crawler principles

- **Cookie persistence**: store cookies under `data/` and reuse them; on expiry, re-login and overwrite.
- **Fail loudly, debug safely**: when parsing breaks, save minimal HTML samples under `data/` (gitignored) and avoid leaking PII in logs.
- **Keep identifiers stable**: this codebase currently keys scores by `(title, chart_type, diff_category)`; avoid introducing unstable keys.

## Code hygiene

- Keep modules small and focused (`config`, `http`, `maimai`, `db`, `discord`).
- Prefer small testable helpers.
- If adding "UI preview" tests, keep them `#[ignore]` and ensure they require explicit env vars (and never log secrets).
- **Always run before committing**:
  - `cargo fmt --all` (format all code)
  - `cargo clippy --all -- -D warnings` (lint with warnings as errors)
