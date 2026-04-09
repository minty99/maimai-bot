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
    curl \
    python3 \
    python3-venv \
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
    # Ubuntu Noble provides the requested libasound2 runtime via the t64 package.
    libasound2t64 \
    libpango-1.0-0 \
    libpangocairo-1.0-0 \
    fonts-dejavu-core \
    && rm -rf /var/lib/apt/lists/*

# Install uv
RUN curl -LsSf https://astral.sh/uv/install.sh | sh

ENV PATH="/root/.local/bin:$PATH"
# Prefer the system Python installed above rather than letting uv download its own
ENV UV_PYTHON_PREFERENCE=only-system
ENV BROWSER_PATH=/usr/local/bin/google-chrome

WORKDIR /app

# Copy discord binary
COPY --from=builder /app/target/release/maistats-discord-bot /usr/local/bin/maistats-discord-bot

# Copy the plot script
COPY scripts/ ./scripts/

# Pre-warm: triggers uv to parse the PEP 723 metadata and install dependencies
# into its cache layer before the first real request hits.
# The script exits with an error (empty stdin), but that is expected — || true suppresses it.
RUN uv run --script scripts/mai_plot.py > /dev/null 2>&1 || true

# Kaleido v1 no longer bundles Chrome, so install a compatible browser into
# the image and expose it at a stable path that Kaleido will always discover.
RUN mkdir -p /opt/plotly/chrome && \
    uv run --with plotly~=6.6 --with kaleido~=1.2 python -c \
    'from pathlib import Path; import plotly.io as pio; chrome_path = Path(pio.get_chrome(path="/opt/plotly/chrome")); symlink_path = Path("/usr/local/bin/google-chrome"); symlink_path.unlink(missing_ok=True); symlink_path.symlink_to(chrome_path)'

CMD ["maistats-discord-bot"]
