CREATE TABLE owned_chart_themes
(
  id INTEGER PRIMARY KEY,
  chart_theme_id INTEGER NOT NULL,
  user_id INTEGER NOT NULL,

  FOREIGN KEY (user_id) REFERENCES users (id)
);

CREATE TABLE selected_chart_themes
(
  user_id INTEGER UNIQUE,
  owned_chart_theme_id INTEGER NOT NULL,

  FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
  FOREIGN KEY (owned_chart_theme_id) REFERENCES owned_chart_themes (id) ON DELETE CASCADE
);
