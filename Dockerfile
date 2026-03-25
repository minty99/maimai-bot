# Multi-stage build for maimai-bot workspace
# Builds the record collector and Discord bot from a single builder stage

# ============================================
# Builder Stage - Compiles entire workspace
# ============================================
FROM rust:1.93-slim as builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    libssl-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml ./
COPY Cargo.lock ./
COPY crates/ ./crates/
COPY maistats-record-collector/ ./maistats-record-collector/
COPY maistats-song-info/ ./maistats-song-info/
COPY maistats-discord-bot/ ./maistats-discord-bot/

# Build entire workspace
RUN cargo build --release

# ============================================
# Target: maistats-record-collector
# ============================================
FROM ubuntu:noble as maistats-record-collector

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy record collector binary
COPY --from=builder /app/target/release/maistats-record-collector /usr/local/bin/maistats-record-collector

# Create data directory
RUN mkdir -p /app/data

EXPOSE 3000

CMD ["maistats-record-collector"]

# ============================================
# Target: maistats-discord-bot
# ============================================
FROM ubuntu:noble as maistats-discord-bot

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy discord binary
COPY --from=builder /app/target/release/maistats-discord-bot /usr/local/bin/maistats-discord-bot

CMD ["maistats-discord-bot"]
