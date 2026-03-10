-- Add play-credit indexing and per-play flags.
PRAGMA foreign_keys = ON;

ALTER TABLE playlogs ADD COLUMN credit_play_count INTEGER;
ALTER TABLE playlogs ADD COLUMN achievement_new_record INTEGER NOT NULL DEFAULT 0;
ALTER TABLE playlogs ADD COLUMN first_play INTEGER NOT NULL DEFAULT 0;

CREATE INDEX IF NOT EXISTS idx_playlogs_credit_play_count ON playlogs(credit_play_count);
CREATE INDEX IF NOT EXISTS idx_playlogs_played_at ON playlogs(played_at);
