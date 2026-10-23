#!/usr/bin/env bash
# Restore a backup created by scripts/backup.sh. Stops the app during the restore.
# Usage: scripts/restore.sh backups/opensyllabus-<stamp>.db backups/storage-<stamp>.tar.gz
set -euo pipefail
DB="${1:?path to opensyllabus-*.db}"
FILES="${2:?path to storage-*.tar.gz}"
read -r -p "This replaces the current database and uploads. Continue? [y/N] " ok
[[ "$ok" == "y" || "$ok" == "Y" ]] || exit 1

docker compose stop opensyllabus
# Use a throwaway container that mounts the same volume.
docker compose run --rm --no-deps -T --entrypoint sh -v "$(realpath "$DB")":/restore/opensyllabus.db:ro -v "$(realpath "$FILES")":/restore/storage.tar.gz:ro opensyllabus -c '
  set -e
  rm -f /data/db/opensyllabus.db /data/db/opensyllabus.db-wal /data/db/opensyllabus.db-shm
  cp /restore/opensyllabus.db /data/db/opensyllabus.db
  rm -rf /data/storage && tar -C /data -xzf /restore/storage.tar.gz
'
docker compose start opensyllabus
echo "Restore complete."
