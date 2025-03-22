CREATE TABLE trade_card_rarities
(
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  weight INTEGER NOT NULL,
  color INTEGER NOT NULL,
  emote_id INTEGER NOT NULL,
  sell_coins INTEGER NOT NULL
);

CREATE TABLE trade_card_authors
(
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  user_id INTEGER NOT NULL
);

CREATE TABLE trade_cards
(
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  quote TEXT NOT NULL,
  rarity_id INTEGER NOT NULL,
  author_id INTEGER NOT NULL,

  FOREIGN KEY (rarity_id) REFERENCES trade_card_rarities (id),
  FOREIGN KEY (author_id) REFERENCES trade_card_authors (id)
);
