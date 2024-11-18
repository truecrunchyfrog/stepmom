CREATE TABLE IF NOT EXISTS users
(
  id INTEGER PRIMARY KEY,
  uid INTEGER UNIQUE NOT NULL,
  coins INTEGER NOT NULL DEFAULT 0
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

  FOREIGN KEY(source_id) REFERENCES message_refs(id),
  FOREIGN KEY(repost_id) REFERENCES message_refs(id),
  FOREIGN KEY(starrer_id) REFERENCES users(id)
);
