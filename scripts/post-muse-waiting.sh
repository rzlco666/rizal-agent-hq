#!/usr/bin/env bash
# Tandai Muse sedang butuh action Rizal di HQ.
# Pakai: ./scripts/post-muse-waiting.sh "Pertanyaan / keputusan yang dibutuhkan"
# Mengirim heartbeat status menunggu-rizal ke HQ lokal (default port 8900).
# HQ yang sudah diperbaiki akan menampilkan Muse = Menunggu Rizal + alert Perlu Action
# di panel "Butuh Rizal" dan badge alert. Untuk kembali normal, kirim status bekerja:
#   curl -X POST http://127.0.0.1:8900/api/heartbeat -H 'Content-Type: application/json' \
#     -d '{"agent":"Muse","status":"bekerja","task":"..."}'
set -euo pipefail
PORT="${HQ_PORT:-8900}"
TASK="${1:-Muse butuh action dari Rizal}"
curl -s -X POST "http://127.0.0.1:$PORT/api/heartbeat" \
  -H 'Content-Type: application/json' \
  -d "$(python3 -c 'import json,sys; print(json.dumps({"agent":"Muse","status":"menunggu-rizal","task":sys.argv[1]}))' "$TASK")"
echo ""
echo "Terkirim ke HQ port $PORT: Muse = menunggu-rizal"
