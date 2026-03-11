# Runtime-only image packaging for prebuilt workspace binaries.

# ============================================
# Target: maistats-song-info
# ============================================
FROM ubuntu:noble as maistats-song-info

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy song info binary
COPY docker-dist/maistats-song-info/maistats-song-info /usr/local/bin/maistats-song-info

# Create data directory
RUN mkdir -p /app/data

EXPOSE 3001

CMD ["maistats-song-info"]

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
COPY docker-dist/maistats-record-collector/maistats-record-collector /usr/local/bin/maistats-record-collector

# Copy migrations
COPY docker-dist/maistats-record-collector/migrations /app/migrations

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
COPY docker-dist/maistats-discord-bot/maistats-discord-bot /usr/local/bin/maistats-discord-bot

CMD ["maistats-discord-bot"]
