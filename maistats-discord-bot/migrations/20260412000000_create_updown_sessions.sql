CREATE TABLE IF NOT EXISTS updown_sessions (
  discord_user_id TEXT PRIMARY KEY,
  thread_channel_id TEXT NOT NULL,
  pick_message_id TEXT NOT NULL,
  current_level_tenths INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
