# Multi-stage build for Rust backend
FROM rust:1.83-slim as builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    libssl-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml ./
COPY Cargo.lock ./
COPY crates/ ./crates/
COPY backend/ ./backend/

# Build backend only
RUN cargo build --release --bin maimai-backend

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/maimai-backend /usr/local/bin/maimai-backend

# Copy migrations
COPY backend/migrations /app/migrations

# Create data directory
RUN mkdir -p /app/data

# Set environment (will be overridden by docker-compose)
ENV DATABASE_URL=sqlite:/app/data/maimai.sqlite3
ENV PORT=3000

EXPOSE 3000

CMD ["maimai-backend"]
