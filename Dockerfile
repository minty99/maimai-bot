# Multi-stage build for maimai-bot workspace
# Builds both backend and discord bot from a single builder stage

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
COPY record-collector-server/ ./record-collector-server/
COPY song-info-server/ ./song-info-server/
COPY personal-discord-bot/ ./personal-discord-bot/

# Build entire workspace (both binaries)
RUN cargo build --release

# ============================================
# Target: maimai-song-info
# ============================================
FROM ubuntu:noble as maimai-song-info

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy song info binary
COPY --from=builder /app/target/release/maimai-song-info /usr/local/bin/maimai-song-info

# Create data directory
RUN mkdir -p /app/data

EXPOSE 3001

CMD ["maimai-song-info"]

# ============================================
# Target: maimai-record-collector-server
# ============================================
FROM ubuntu:noble as maimai-record-collector-server

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy record collector binary
COPY --from=builder /app/target/release/record-collector-server /usr/local/bin/record-collector-server

# Copy migrations
COPY record-collector-server/migrations /app/migrations

# Create data directory
RUN mkdir -p /app/data

EXPOSE 3000

CMD ["record-collector-server"]

# ============================================
# Target: maimai-personal-discord-bot
# ============================================
FROM ubuntu:noble as maimai-personal-discord-bot

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy discord binary
COPY --from=builder /app/target/release/personal-discord-bot /usr/local/bin/personal-discord-bot

CMD ["personal-discord-bot"]
