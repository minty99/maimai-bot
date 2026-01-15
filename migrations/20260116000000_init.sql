-- Core tables for single-user local storage.
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS songs (
  song_key TEXT PRIMARY KEY NOT NULL,
  title TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS scores (
  song_key TEXT NOT NULL,
  chart_type TEXT NOT NULL, -- 'STD' | 'DX'
  diff INTEGER NOT NULL, -- 0..4
  achievement_percent REAL,
  rank TEXT,
  fc TEXT,
  sync TEXT,
  dx_score INTEGER,
  dx_score_max INTEGER,
  source_idx TEXT,
  scraped_at INTEGER NOT NULL,
  PRIMARY KEY (song_key, chart_type, diff),
  FOREIGN KEY (song_key) REFERENCES songs(song_key)
);

CREATE INDEX IF NOT EXISTS idx_scores_scraped_at ON scores(scraped_at);

CREATE TABLE IF NOT EXISTS playlogs (
  playlog_idx TEXT PRIMARY KEY NOT NULL,
  played_at TEXT,
  track INTEGER,
  song_key TEXT NOT NULL,
  title TEXT NOT NULL,
  chart_type TEXT NOT NULL, -- 'STD' | 'DX'
  diff INTEGER,
  achievement_percent REAL,
  score_rank TEXT,
  fc TEXT,
  sync TEXT,
  dx_score INTEGER,
  dx_score_max INTEGER,
  scraped_at INTEGER NOT NULL,
  FOREIGN KEY (song_key) REFERENCES songs(song_key)
);

CREATE INDEX IF NOT EXISTS idx_playlogs_scraped_at ON playlogs(scraped_at);

