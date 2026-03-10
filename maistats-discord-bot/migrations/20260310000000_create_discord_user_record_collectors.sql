CREATE TABLE IF NOT EXISTS discord_user_record_collectors (
  discord_user_id TEXT PRIMARY KEY,
  record_collector_server_url TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
