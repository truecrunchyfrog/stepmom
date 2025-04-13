CREATE TABLE owned_chart_themes
(
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  user_id INTEGER NOT NULL,

  FOREIGN KEY (user_id) REFERENCES users (id)
);
