-- Core tables for single-user local storage.
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS scores (
  title TEXT NOT NULL,
  chart_type TEXT NOT NULL, -- 'STD' | 'DX'
  diff_category TEXT NOT NULL, -- 'BASIC' | 'ADVANCED' | 'EXPERT' | 'MASTER' | 'Re:MASTER'
  level TEXT NOT NULL, -- e.g. '13+'
  achievement_percent REAL,
  rank TEXT,
  fc TEXT,
  sync TEXT,
  dx_score INTEGER,
  dx_score_max INTEGER,
  source_idx TEXT,
  scraped_at INTEGER NOT NULL,
  PRIMARY KEY (title, chart_type, diff_category)
);

CREATE INDEX IF NOT EXISTS idx_scores_scraped_at ON scores(scraped_at);

CREATE TABLE IF NOT EXISTS playlogs (
  playlog_idx TEXT PRIMARY KEY NOT NULL,
  played_at TEXT,
  track INTEGER,
  title TEXT NOT NULL,
  chart_type TEXT NOT NULL, -- 'STD' | 'DX'
  diff_category TEXT, -- 'BASIC' | 'ADVANCED' | 'EXPERT' | 'MASTER' | 'Re:MASTER'
  level TEXT, -- e.g. '13+'
  achievement_percent REAL,
  score_rank TEXT,
  fc TEXT,
  sync TEXT,
  dx_score INTEGER,
  dx_score_max INTEGER,
  scraped_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_playlogs_scraped_at ON playlogs(scraped_at);
