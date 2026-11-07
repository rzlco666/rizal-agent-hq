# Ukur Performa HQ — log `[perf]` + `/api/health` diperdalam

Fase UKUR (bukan perbaikan). Lihat riset: `~/workspace/work/hq-performance-research-2026-11-07.md`.

## Log `[perf]`

Satu baris per siklus refresh (30 dtk) di stdout service:

```
[perf] refresh total=1234ms collect=1100ms sys=12ms docker=30ms ts=210ms git=340ms ps=45ms tmux=25ms omp_stats=0ms omp_usage=0ms sessions=180ms sources=900ms alert=2ms db=15ms
```

- `total` = seluruh `refresh()` (kolektor + evaluasi alert + tulis SQLite + broadcast).
- `collect` = `hq_collect::collect_snapshot` saja.
- `sys` = `/proc` + `df`; `docker` = `docker ps -a`; `ts` = `tailscale status`;
  `git` = `git log -1` per repo di `~/projects` (sekuensial); `ps` = `ps -eo`;
  `tmux` = `list-sessions` + `list-panes`.
- `omp_stats` / `omp_usage` = `omp stats --json` / `omp usage`; **0 = cache
  masih segar** (stats TTL 60 dtk, usage TTL 5 mnt) sehingga perintah tidak jalan.
- `sessions` = baca + parse file sesi OMP (maks 40 file, 24 jam terakhir).
- `sources` = seluruh loop registry sumber (omp, cursor, Muse, coding-agents, mcp).
- `alert` = evaluasi + rekonsiliasi alert; `db` = tulis `alert_state` + `snapshots`.

Satu baris per pekerjaan perapi LLM (utas latar, bukan siklus refresh):

```
[perf] llm task=<id> model=opencode-go/beam-free dur_ms=12345 ok=true
```

## `/api/health` (diperdalam)

```json
{
  "ok": true, "at": 1794015000000, "app": "rizal-agent-hq",
  "version": "2.0.0", "uptime_secs": 3600,
  "db": { "ok": true, "error": null, "snapshots": 10126, "llm_summaries": 30 },
  "stocks": {
    "fresh": true, "symbols": 4, "fresh_symbols": 4,
    "age_secs": 3600, "last_fetch": "2026-11-07T15:00:00+07:00", "source": "EODHD-Live"
  },
  "mem": { "rss_mb": 168.0, "peak_mb": 1024.0 },
  "disk": { "percent": 87.0 },
  "perf": { "refresh_total_ms": 1234, "...": "..." }
}
```

- `ok` = liveness proses (handler jalan). `db.ok=false` + `error` bila query
  gagal atau mutex DB sibuk saat dicek (try_lock, tanpa blokir).
- `stocks.fresh` = ada simbol yang berhasil diambil hari ini (WIB).
  `age_secs` = detik sejak fetch terakhir; `null` bila belum pernah fetch.
- `mem` dari `/proc/self/status` (VmRSS / VmHWM). `disk.percent` dari snapshot
  terakhir. `perf` = hasil ukur siklus refresh terakhir (sama seperti log).
- Kuma bisa keyword-check `"stocks_fresh":true` (M3 dari riset).

## Sesi profiling (staging, bukan service live)

Binary staging jalan di port + data dir terpisah — JANGAN sentuh service live
atau `data/`:

```sh
# tokio-console: butuh fitur `console` + cfg tokio_unstable
RUSTFLAGS="--cfg tokio_unstable" cargo build --release --features console
HQ_PORT=8901 HQ_DATA=/tmp/hq-perf-data HQ_CONSOLE=1 ./target/release/rizal-agent-hq
tokio-console   # di tmux; Tasks view + Resources view (mutex)
```

```sh
# flamegraph: `perf` TIDAK tersedia di WSL (sudo butuh password).
# Alternatif yang jalan: samply (perf_event_open, paranoid=2).
cargo install samply
HQ_PORT=8902 HQ_DATA=/tmp/hq-perf-data2 samply record --save-only -o /tmp/hq-perf.json -- ./target/release/rizal-agent-hq
```

## Yang TIDAK diubah fase ini

Tidak ada `spawn_blocking`, tidak ada cache kolektor baru, tidak ada WAL/indeks,
tidak ada kompresi, tidak ada `MemoryMax`. Semua itu menunggu hasil ukur.
