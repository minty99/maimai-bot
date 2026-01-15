# Agent Guidelines (maimai-bot)

This repository is a **single-user** Rust app that logs in to **maimaidx-eng.com** (SEGA ID) to crawl personal records, store them locally, and later expose queries via a Discord bot.

## Non-negotiables
- **Single user only**: do not add multi-user concepts (accounts table, user scoping, etc.).
- **Secrets never committed**: `.env` and `data/` are always gitignored; do not print credentials or full cookie contents to logs.
- **Dependency management**: prefer `cargo add` / `cargo remove` over editing `Cargo.toml` directly (use the latest compatible version by default).
- **Error handling**: use `eyre` (`eyre::Result`, `WrapErr`) consistently.
- **Validation**: prefer `cargo clippy` (and `cargo fmt`) over `cargo check` for routine validation.

## Crawler principles
- **Cookie persistence**: store cookies under `data/` and reuse them; on expiry, re-login and overwrite.
- **Fail loudly, debug safely**: when parsing/login breaks, save minimal HTML samples under `data/` (gitignored) and avoid leaking PII in logs.
- **Keep keys stable**: start with title-based `song_key` if needed, but ensure it is never empty; migrate to a real ID only when a stable site-provided identifier is found.

## Code hygiene
- Keep modules small and focused (`config`, `http`, `maimai`, `db`).
- Prefer simple, testable functions; live integration tests may require real credentials/network and should be written with that in mind (timeouts, no secret logging).
