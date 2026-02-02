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
COPY backend/ ./backend/
COPY discord/ ./discord/

# Build entire workspace (both binaries)
RUN cargo build --release

# ============================================
# Target: maimai-backend
# ============================================
FROM ubuntu:noble as maimai-backend

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy backend binary
COPY --from=builder /app/target/release/maimai-backend /usr/local/bin/maimai-backend

# Copy migrations
COPY backend/migrations /app/migrations

# Create data directory
RUN mkdir -p /app/data

# Environment defaults
ENV DATABASE_URL=sqlite:/app/data/maimai.sqlite3
ENV BACKEND_PORT=3000

EXPOSE 3000

CMD ["maimai-backend"]

# ============================================
# Target: maimai-discord
# ============================================
FROM ubuntu:noble as maimai-discord

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy discord binary
COPY --from=builder /app/target/release/maimai-discord /usr/local/bin/maimai-discord

# Environment defaults
ENV BACKEND_URL=http://backend:3000

CMD ["maimai-discord"]
