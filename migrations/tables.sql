CREATE TABLE IF NOT EXISTS users
(
  id INTEGER PRIMARY KEY,
  uid INTEGER UNIQUE NOT NULL,
  coins INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS coin_transactions
(
  id INTEGER PRIMARY KEY,
  user_id INTEGER NOT NULL,
  coins_diff INTEGER NOT NULL,

  FOREIGN KEY (user_id) REFERENCES users (id)
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

  FOREIGN KEY (source_id) REFERENCES message_refs (id),
  FOREIGN KEY (repost_id) REFERENCES message_refs (id),
  FOREIGN KEY (starrer_id) REFERENCES users (id)
);
