#!/usr/bin/env bash
# Rizal Agent HQ — pemasangan plug & play (Linux + systemd user).
# Membangun dari sumber, memasang ke ~/rizal-agent-hq, menyalakan service.
set -euo pipefail
cd "$(dirname "$0")/.."
ROOT="$(pwd)"
DEST="${HQ_DEST:-$HOME/rizal-agent-hq}"
PORT="${HQ_PORT:-8900}"

echo "== Rizal Agent HQ — install =="
command -v cargo >/dev/null || { echo "Butuh Rust/cargo: https://rustup.rs"; exit 1; }

echo "-- build release…"
export PATH="$HOME/.cargo/bin:$PATH"
cargo build --release -p hq-server

echo "-- pasang ke $DEST"
mkdir -p "$DEST/bin" "$DEST/data"
cp target/release/rizal-agent-hq "$DEST/bin/"
cp -r web "$DEST/"
cp -r scripts "$DEST/"

echo "-- pasang systemd user service…"
mkdir -p "$HOME/.config/systemd/user"
sed "s|%h/rizal-agent-hq|$DEST|g; s|HQ_PORT=8900|HQ_PORT=$PORT|" systemd/rizal-agent-hq.service \
  > "$HOME/.config/systemd/user/rizal-agent-hq.service"
sed "s|%h/rizal-agent-hq|$DEST|g" systemd/rizal-agent-hq-report.service \
  > "$HOME/.config/systemd/user/rizal-agent-hq-report.service"
cp systemd/rizal-agent-hq-report.timer "$HOME/.config/systemd/user/"
systemctl --user daemon-reload
systemctl --user enable --now rizal-agent-hq.service
systemctl --user enable --now rizal-agent-hq-report.timer
# Agar tetap jalan setelah logout (opsional, butuh hak):
loginctl enable-linger "$USER" 2>/dev/null || echo "   (tips: jalankan 'loginctl enable-linger $USER' agar service hidup setelah logout)"

sleep 2
if curl -sf "http://127.0.0.1:$PORT/api/health" >/dev/null; then
  echo ""
  echo "✅ Rizal Agent HQ jalan: http://127.0.0.1:$PORT/  (dari Windows: http://localhost:$PORT/)"
else
  echo "Service terpasang tapi health-check belum menjawab; cek: systemctl --user status rizal-agent-hq"
fi
echo "Folder proyek tetap di: $ROOT"
