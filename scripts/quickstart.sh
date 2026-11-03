#!/usr/bin/env bash
# Rizal Agent HQ — jalan cepat tanpa install permanen.
# Build release hanya jika biner belum ada atau Cargo.toml lebih baru,
# lalu jalan di foreground. Ctrl+C untuk berhenti.
# Untuk service permanen, pakai ./scripts/install.sh.
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
command -v cargo >/dev/null || { echo "Butuh Rust/cargo: https://rustup.rs"; exit 1; }
if [ ! -x target/release/rizal-agent-hq ] || [ Cargo.toml -nt target/release/rizal-agent-hq ]; then
  echo "-- build release…"
  cargo build --release -p hq-server
fi
PORT="${HQ_PORT:-8900}"
echo "Rizal Agent HQ jalan di http://127.0.0.1:$PORT/  (dari Windows: http://localhost:$PORT/)"
echo "Tekan Ctrl+C untuk berhenti. Service permanen: ./scripts/install.sh"
HQ_WEB="${HQ_WEB:-web}" HQ_DATA="${HQ_DATA:-data}" exec target/release/rizal-agent-hq
