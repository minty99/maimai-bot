-- Rebuild scores table without backward compatibility.
DROP TABLE IF EXISTS scores;

CREATE TABLE scores (
  title TEXT NOT NULL,
  chart_type TEXT NOT NULL,
  diff_category TEXT NOT NULL,
  achievement_x10000 INTEGER,
  rank TEXT,
  fc TEXT,
  sync TEXT,
  dx_score INTEGER,
  dx_score_max INTEGER,
  last_played_at TEXT,
  play_count INTEGER,
  PRIMARY KEY (title, chart_type, diff_category)
);
