-- Rename playlog credit identifier column without preserving backward compatibility.
DROP INDEX IF EXISTS idx_playlogs_credit_play_count;

CREATE TABLE playlogs_new (
  played_at_unixtime INTEGER PRIMARY KEY NOT NULL,
  played_at TEXT,
  track INTEGER,
  title TEXT NOT NULL,
  chart_type TEXT NOT NULL,
  diff_category TEXT,
  achievement_x10000 INTEGER,
  score_rank TEXT,
  fc TEXT,
  sync TEXT,
  dx_score INTEGER,
  dx_score_max INTEGER,
  credit_id INTEGER,
  achievement_new_record INTEGER NOT NULL DEFAULT 0
);

INSERT INTO playlogs_new (
  played_at_unixtime,
  played_at,
  track,
  title,
  chart_type,
  diff_category,
  achievement_x10000,
  score_rank,
  fc,
  sync,
  dx_score,
  dx_score_max,
  credit_id,
  achievement_new_record
)
SELECT
  played_at_unixtime,
  played_at,
  track,
  title,
  chart_type,
  diff_category,
  achievement_x10000,
  score_rank,
  fc,
  sync,
  dx_score,
  dx_score_max,
  credit_play_count,
  achievement_new_record
FROM playlogs;

DROP TABLE playlogs;
ALTER TABLE playlogs_new RENAME TO playlogs;

CREATE INDEX IF NOT EXISTS idx_playlogs_credit_id ON playlogs(credit_id);
CREATE INDEX IF NOT EXISTS idx_playlogs_played_at ON playlogs(played_at);
