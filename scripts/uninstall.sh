#!/usr/bin/env bash
# Rizal Agent HQ — copot pemasangan (data di ~/rizal-agent-hq/data TIDAK dihapus otomatis).
set -euo pipefail
DEST="${HQ_DEST:-$HOME/rizal-agent-hq}"
systemctl --user disable --now rizal-agent-hq.service 2>/dev/null || true
rm -f "$HOME/.config/systemd/user/rizal-agent-hq.service"
systemctl --user daemon-reload
echo "Service dicabut. Folder pemasangan masih ada di $DEST (hapus manual bila perlu, termasuk data/)."
