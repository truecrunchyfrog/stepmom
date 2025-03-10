CREATE TABLE IF NOT EXISTS users
(
  id INTEGER PRIMARY KEY,
  uid INTEGER UNIQUE NOT NULL
);

CREATE TABLE IF NOT EXISTS study_sessions
(
  id INTEGER PRIMARY KEY,
  user_id INTEGER NOT NULL,
  coin_reward_id INTEGER NULL,

  length INTEGER NOT NULL,
  video_length INTEGER NOT NULL CHECK(video_length <= length),

  ended INTEGER NOT NULL DEFAULT(UNIXEPOCH()),

  FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
  FOREIGN KEY (coin_reward_id) REFERENCES coin_transactions (id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS coin_transactions
(
  id INTEGER PRIMARY KEY,
  user_id INTEGER NOT NULL,

  coins_diff INTEGER NOT NULL,
  timestamp INTEGER NOT NULL DEFAULT(UNIXEPOCH()),

  FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS message_refs
(
  id INTEGER PRIMARY KEY,
  channel_id INTEGER NOT NULL,
  message_id INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS starred_messages
(
  source_id INTEGER NOT NULL UNIQUE,
  repost_id INTEGER NOT NULL UNIQUE,
  starrer_id INTEGER NOT NULL,
  content TEXT NOT NULL,

  FOREIGN KEY (source_id) REFERENCES message_refs (id) ON DELETE CASCADE,
  FOREIGN KEY (repost_id) REFERENCES message_refs (id) ON DELETE CASCADE,
  FOREIGN KEY (starrer_id) REFERENCES users (id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS leaderboard_optout
(
  user_id INTEGER NOT NULL UNIQUE,

  FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS msg_sets (id INTEGER PRIMARY KEY);
CREATE TABLE IF NOT EXISTS msg_set_items
(
  msg_set_id INTEGER NOT NULL,
  message_ref_id INTEGER NOT NULL,

  PRIMARY KEY (msg_set_id, message_ref_id),

  FOREIGN KEY (msg_set_id) REFERENCES msg_sets (id) ON DELETE CASCADE,
  FOREIGN KEY (message_ref_id) REFERENCES message_refs (id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS guild_sent_dm_messages
(
  user_id INTEGER NOT NULL,
  msg_set_id INTEGER NOT NULL,

  PRIMARY KEY (user_id, msg_set_id),

  FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
  FOREIGN KEY (msg_set_id) REFERENCES msg_sets (id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS study_result_preferences
(
  user_id INTEGER NOT NULL UNIQUE,
  /*
    0 = off
    1 = dm
    2 = guild
   */
  mode INT NOT NULL CHECK(mode IN (0, 1, 2)),

  FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS rewards
(
  id INTEGER PRIMARY KEY,
  user_id INTEGER NOT NULL,

  description VARCHAR(50) NOT NULL,
  reason VARCHAR(50) NOT NULL,
  received INTEGER NOT NULL DEFAULT(UNIXEPOCH()),

  FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS boosters
(
  id INTEGER PRIMARY KEY,
  user_id INTEGER NOT NULL,
  -- Booster multiplier in percentage. 150 for 1.5x booster.
  multiplier INT NOT NULL,
  expiration INTEGER NOT NULL,

  FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS video_rewards_time_left
(
  user_id INTEGER NOT NULL UNIQUE,
  time_left INTEGER NOT NULL,

  FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS bumps
(
  user_id INTEGER,
  timestamp INTEGER NOT NULL DEFAULT(UNIXEPOCH()),

  FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE SET NULL
);
