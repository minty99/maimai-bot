-- Remove song-info-owned columns from record collector storage.
DROP INDEX IF EXISTS idx_scores_scraped_at;
DROP INDEX IF EXISTS idx_playlogs_scraped_at;

CREATE TABLE scores_new (
  title TEXT NOT NULL,
  chart_type TEXT NOT NULL,
  diff_category TEXT NOT NULL,
  achievement_x10000 INTEGER,
  rank TEXT,
  fc TEXT,
  sync TEXT,
  dx_score INTEGER,
  dx_score_max INTEGER,
  PRIMARY KEY (title, chart_type, diff_category)
);

INSERT INTO scores_new (
  title,
  chart_type,
  diff_category,
  achievement_x10000,
  rank,
  fc,
  sync,
  dx_score,
  dx_score_max
)
SELECT
  title,
  chart_type,
  diff_category,
  achievement_x10000,
  rank,
  fc,
  sync,
  dx_score,
  dx_score_max
FROM scores;

DROP TABLE scores;
ALTER TABLE scores_new RENAME TO scores;

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
  credit_play_count INTEGER,
  achievement_new_record INTEGER NOT NULL DEFAULT 0,
  first_play INTEGER NOT NULL DEFAULT 0
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
  credit_play_count,
  achievement_new_record,
  first_play
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
  achievement_new_record,
  first_play
FROM playlogs;

DROP TABLE playlogs;
ALTER TABLE playlogs_new RENAME TO playlogs;

CREATE INDEX IF NOT EXISTS idx_playlogs_credit_play_count ON playlogs(credit_play_count);
CREATE INDEX IF NOT EXISTS idx_playlogs_played_at ON playlogs(played_at);
