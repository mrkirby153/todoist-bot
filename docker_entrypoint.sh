#!/usr/bin/env bash

set -e


echo "Syncing emojis..."
/app/emoji-sync --in-dir /app/emoji --content-type image/png --out-file /app/emojis.json

echo "Starting bot..."
exec /app/todoist-bot