#!/usr/bin/env bash
# Laporan harian Rizal Agent HQ — dirender dari /api/state menjadi Markdown.
# Dipanggil systemd timer 23:55 WIB; bisa juga dijalankan manual.
set -euo pipefail
PORT="${HQ_PORT:-8900}"
DEST="${HQ_DEST:-$HOME/rizal-agent-hq}"
OUT_DIR="$DEST/data/reports"
mkdir -p "$OUT_DIR"
DATE="$(date +%F)"
OUT="$OUT_DIR/$DATE.md"
curl -s -m 20 "http://127.0.0.1:$PORT/api/state" | python3 - "$OUT" << 'PYEOF'
import json, sys, datetime
s = json.load(sys.stdin)
out = sys.argv[1]
def tok(n): return f"{n/1e6:.2f} jt" if n >= 1e6 else (f"{n/1e3:.1f} rb" if n >= 1e3 else str(n))
def usd(n): return f"${n:.2f}"
lines = [f"# Laporan Harian Rizal Agent HQ — {datetime.date.today().isoformat()}", ""]
u = s["total_usage"]
lines += ["## Ringkasan",
          f"- Request OMP tercatat: **{u['requests']}** (gagal {u['failed']})",
          f"- Token: masuk {tok(u['input_tokens'])}, keluar {tok(u['output_tokens'])}, cache {tok(u['cache_tokens'])}",
          f"- Biaya TUNAI: **{usd(u['cash_usd'])}** · bayangan (langganan): {usd(u['shadow_usd'])}",
          f"- Estimasi saldo DeepSeek: **{usd(s['deepseek_balance_est']) if s.get('deepseek_balance_est') is not None else '—'}**", ""]
lines.append("## Agent")
for a in s["agents"]:
    au = a["usage"]
    lines.append(f"- {a['emoji']} **{a['name']}** ({a['status']}) — {a['activity']} "
                 f"[token {tok(au['input_tokens']+au['output_tokens'])}, {usd(au['cash_usd']+au['shadow_usd'])}]")
lines += ["", "## Tugas terakhir"]
for t in s["tasks"][:10]:
    lines.append(f"- ({t['phase']}) {t['title']} — {t['detail']}")
aktif = [a for a in s["alerts"] if a["state"] == "aktif"]
lines += ["", "## Alert aktif"]
lines += [f"- [{a['level']}] {a['title']} — {a['needs']}" for a in aktif] or ["- Tidak ada."]
lines += ["", "## Kuota menonjol"]
for q in s["quotas"]:
    if q.get("percent") is not None and q["percent"] >= 80:
        lines.append(f"- {q['provider']} · {q['label']}: {q['percent']}% ({q.get('extra','')})")
open(out, "w").write("\n".join(lines) + "\n")
print(out)
PYEOF
echo "Laporan tertulis: $OUT"
