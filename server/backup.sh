#!/usr/bin/env bash
# Copy the server key pair, peer database and compose file from the VPS to a
# local tarball. Run from any machine with SSH access as root.
#
#   server/backup.sh [ssh-host] [output-dir]
#
# Keep the tarball somewhere safe and private: id_ed25519 is the server's
# private key.
set -euo pipefail

HOST="${1:-root@rustdesk.arenna38.com}"
OUT_DIR="${2:-.}"
STAMP="$(date +%Y%m%d-%H%M%S)"
OUT="${OUT_DIR}/arenna-remote-server-${STAMP}.tar.gz"

# SQLite runs in WAL mode; copying db + -wal + -shm together is consistent
# enough for a restore (hbbs replays the WAL on start).
ssh -o BatchMode=yes "$HOST" \
  "tar czf - -C /opt/rustdesk-server data/id_ed25519 data/id_ed25519.pub data/db_v2.sqlite3 data/db_v2.sqlite3-wal data/db_v2.sqlite3-shm docker-compose.yml" \
  > "$OUT"
chmod 600 "$OUT"
echo "Backup written to $OUT"
tar tzf "$OUT"
