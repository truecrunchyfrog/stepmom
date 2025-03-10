#!/usr/bin/env bash

[[ -n "$DCE" ]] || { echo 'Missing $DCE (DiscordChatExport.Cli program)'; exit 1; }
[[ -n "$DB" ]] || { echo 'Missing $DB (SQLite database filename)'; exit 1; }
[[ -n "$TOKEN" ]] || { echo 'Missing $TOKEN (Discord token)'; exit 1; }
[[ -n "$CHANNEL_ID" ]] || { echo 'Missing $CHANNEL_ID (Archive channel ID)'; exit 1; }

DUMP_FILE_NAME='discord_archive_channel.json'
INTERMEDIATE_FILE_NAME='transformed_data.json'

if [[ ! -f "$DUMP_FILE_NAME" || $(read -p "Dump already exists. Ignore and download new? (y/N) " yn && [[ $yn == [yY] ]]) ]]; then
  echo 'Exporting from Discord channel...'
  "$DCE" export --fuck-russia --utc -t "$TOKEN" -f Json -c "$CHANNEL_ID" -o "$DUMP_FILE_NAME"

  echo 'Downloaded all channel content.'
else
  echo 'Using existing cached dump!'
fi

echo 'Transforming content...'

jq '
[.messages[] | {
  uid: (.content | match("`?(\\d+) - \\w+`?").captures[0].string),
  length: (
    (.embeds[0].description | match("Studied for `.*?(\\d+)h.*`").captures[].string // "0" | tonumber) * 60 * 60 +
    (.embeds[0].description | match("Studied for `.*?(\\d+)m.*`").captures[].string // "0" | tonumber) * 60 +
    (.embeds[0].description | match("Studied for `.*?(\\d+)s.*`").captures[].string // "0" | tonumber)
  ),
  video: (.embeds[0].description | contains("Camera/screenshare")),
  timestamp:
    .timestamp
    | match("(.*)\\..*").captures[0].string
    | (. + "Z")
    | fromdate
}]' "$DUMP_FILE_NAME" > "$INTERMEDIATE_FILE_NAME"

echo 'Transformed content.'

echo 'Generating queries and inserting rows into database, and creating any missing users...'

sqlite3 "$DB" <<< $(jq -r '
map(
  ".print Inserting session for and ensuring user exists: " + .uid + "\n" +
  "INSERT OR IGNORE INTO users VALUES (NULL, " + .uid + ");" +
  "INSERT INTO study_sessions VALUES (" +

  ([
    "NULL",
    "(SELECT id FROM users WHERE uid = " + .uid + ")",
    "NULL",
    (.length | tostring),
    (if .video then (.length | tostring) else "0" end),
    (.timestamp | tostring)
  ]
  | join(",")) +

  ");"

) | join("\n")
' "$INTERMEDIATE_FILE_NAME")


echo "Study session data migrated from Discord to database $DB!"

read -p 'Delete cache files? (y/N) ' yn && [[ $yn == [yY] ]] && rm "$DUMP_FILE_NAME" "$INTERMEDIATE_FILE_NAME"
