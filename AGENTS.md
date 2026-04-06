# Agent Guidelines (maimai-bot)

This repo is centered on Rust services that log in to **maimaidx-eng.com** (SEGA ID), crawl data, store it locally (SQLite), expose it via APIs/Discord, and ship a separate `apps/maistats` frontend.

The goal of this file is to keep future changes consistent with the current implementation and constraints.

## Non-negotiables

- **Record collector remains single-user**: do not add multi-user concepts to the crawler / record-collector data model. The Discord bot may store a Discord-user-to-record-collector mapping, but record data itself stays unscoped and per collector instance.
- **Secrets never committed**: `.env` and `data/` are always gitignored; never print credentials or cookie contents.
- **Dependency management**: use `cargo add` / `cargo remove` (avoid hand-editing `Cargo.toml`).
- **Error handling**: use `eyre` (`eyre::Result`, `WrapErr`) consistently.
- **Validation**: prefer `cargo fmt`, `cargo clippy`, `cargo test`.
- **Version bumps**: when bumping `[workspace.package] version` in `Cargo.toml`, add a matching entry to the `CHANGELOG` constant in `maistats-discord-bot/src/commands.rs` (format: `("x.y.z", "one-line English description of what changed")`). The bot shows this changelog to users whose record collector is out of date.

## Runtime architecture (current)

- Frontend app: `apps/maistats`
  - Vite + React app deployed to Cloudflare Pages
  - Uses root npm workspaces; prefer running npm commands from the repo root
  - Consumes static song database assets and `maistats-record-collector` via `SONG_DATABASE_URL` and `RECORD_COLLECTOR_SERVER_URL`

- CLI entrypoint: `src/main.rs` (clap). Key subcommands:
  - `auth check|login`
  - `fetch url --url <URL> --out <FILE>` (authenticated fetch, raw bytes)
  - `crawl scores|recent|song-detail|player-data` (parse to JSON; no DB)
  - `db init|sync-scores|sync-recent`
  - `bot run` (Discord bot)
- Discord bot: `maistats-discord-bot/src/main.rs`
  - On startup:
    - runs bot-local SQLite migrations
    - registers slash commands globally
    - sends a startup summary DM only to `DISCORD_DEV_USER_ID`
  - Per-user collector selection:
    - `SONG_DATABASE_URL` is shared globally
    - each Discord user registers exactly one `maistats-record-collector` base URL with `/register <url>`
    - the mapping is persisted in the bot's own SQLite DB and survives restarts
  - Slash commands:
    - `/register <url>`: validate readiness + player API, upsert caller's collector URL, and reply ephemerally
    - `/mai-score <title|alias>`: exact title or registered alias match
      - exact title/alias match: resolve to the canonical song, then show that title
      - ambiguous alias: show duplicate candidates
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
- **Visibility first**: default to private, use `pub(crate)` for crate-internal sharing, and keep `pub` only for true cross-crate API.
- If adding "UI preview" tests, keep them `#[ignore]` and ensure they require explicit env vars (and never log secrets).
- **Always run before committing**:
  - `cargo fmt --all` (format all code)
  - `cargo clippy --all -- -D warnings` (lint with warnings as errors)

## Commit discipline

- **Use Conventional Commits subject line**: `<type>(<scope>): <summary>` (imperative, lowercase type).
  - Preferred types: `feat`, `fix`, `refactor`, `docs`, `test`, `ci`, `chore`.
  - Scope should map to component names in this repo (e.g. `discord`, `maistats-record-collector`, `maistats-song-info`, `models`, `agents`).
  - Examples:
    - `fix(maistats-record-collector): handle all matching musicDetail indexes`
    - `docs(agents): define commit message convention`
- **Atomic commits**: split commits by meaning (one logical change per commit), and avoid bundling unrelated modifications.
- **Co-author trailer required**: every commit message must include an agent co-author trailer in the commit body.
  - OpenAI agents: `Co-authored-by: <Agent Model Name> <noreply@openai.com>` (e.g., `Co-authored-by: GPT-5.3 Codex <noreply@openai.com>`)
  - Anthropic agents: `Co-authored-by: <Agent Model Name> <noreply@anthropic.com>` (e.g., `Co-authored-by: Claude Sonnet 4.5 <noreply@anthropic.com>`)
- **No escaped newlines in commit body**: never write literal `\n` in commit messages. Use real line breaks only.
  - Prefer `git commit -m "<subject>" -m "<body line 1>" -m "<body line 2>"` (multiple `-m`) or `git commit` with an editor.
- **Trailer recognition check (mandatory after each commit)**:
  - `git log -1 --format='%B'` must show a standalone `Co-authored-by:` trailer line.
  - If the trailer is missing or `\n` appears literally, fix immediately with `git commit --amend -m ...` (using real newlines).
