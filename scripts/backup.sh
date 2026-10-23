#!/usr/bin/env bash
# Consistent online backup of the SQLite database and uploaded files.
# Usage: scripts/backup.sh [output-dir]      (default: ./backups)
set -euo pipefail
OUT_DIR="${1:-./backups}"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
mkdir -p "$OUT_DIR"

# 1. SQLite snapshot via VACUUM INTO (safe while the server is running).
docker compose exec -T opensyllabus opensyllabus backup --output "/data/backups/opensyllabus-$STAMP.db"
docker compose cp "opensyllabus:/data/backups/opensyllabus-$STAMP.db" "$OUT_DIR/opensyllabus-$STAMP.db"
docker compose exec -T opensyllabus rm -f "/data/backups/opensyllabus-$STAMP.db"

# 2. Uploaded files.
docker compose exec -T opensyllabus tar -C /data -czf - storage > "$OUT_DIR/storage-$STAMP.tar.gz"

echo "Backup written to $OUT_DIR (opensyllabus-$STAMP.db, storage-$STAMP.tar.gz)"
