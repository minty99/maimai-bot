# Multi-stage build for maimai-bot workspace
# Builds the record collector and Discord bot from a single builder stage

# ============================================
# Builder Stage - Compiles entire workspace
# ============================================
# rust:1.93-slim currently tracks Debian trixie; keep runtime stages on the same
# distro family/release so the Rust binaries and container libc stay aligned.
FROM rust:1.93-slim AS builder

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
FROM debian:trixie-slim AS maistats-record-collector

ARG DEBIAN_FRONTEND=noninteractive

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
FROM debian:trixie-slim AS maistats-discord-bot

ARG DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    libssl3 \
    python3 \
    chromium \
    # kaleido / Chromium system dependencies
    libnss3 \
    libatk1.0-0 \
    libatk-bridge2.0-0 \
    libcups2 \
    libcairo2 \
    libdrm2 \
    libxcomposite1 \
    libxdamage1 \
    libxfixes3 \
    libxrandr2 \
    libgbm1 \
    libxkbcommon0 \
    libasound2 \
    libpango-1.0-0 \
    libpangocairo-1.0-0 \
    fonts-dejavu-core \
    && rm -rf /var/lib/apt/lists/*

RUN curl -LsSf https://astral.sh/uv/install.sh | sh

ENV PATH="/root/.local/bin:$PATH"
ENV UV_PYTHON_PREFERENCE=only-system
ENV BROWSER_PATH=/usr/bin/chromium

WORKDIR /app

# Copy discord binary
COPY --from=builder /app/target/release/maistats-discord-bot /usr/local/bin/maistats-discord-bot

# Copy the plot script
COPY scripts/ ./scripts/

# Validate the script end-to-end during image build with a minimal payload.
RUN printf '%s' '{"points":[{"achievement":100.0,"level_tenths":130}],"x_min":97.0}' | \
    uv run --script scripts/mai_plot.py > /dev/null

CMD ["maistats-discord-bot"]
