# Multi-stage build for Rust Discord bot
FROM rust:1.83-slim as builder

WORKDIR /app

RUN apt-get update && apt-get install -y \
    libssl-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml ./
COPY Cargo.lock ./
COPY crates/ ./crates/
COPY discord/ ./discord/

RUN cargo build --release --bin maimai-discord

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/maimai-discord /usr/local/bin/maimai-discord

ENV BACKEND_URL=http://backend:3000

CMD ["maimai-discord"]
