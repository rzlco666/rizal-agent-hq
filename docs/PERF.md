# Ukur Performa HQ — log `[perf]` + `/api/health` diperdalam

Fase UKUR (bukan perbaikan) + fase PERBAIKI B1–B5 (di bawah). Lihat riset: `~/workspace/work/hq-performance-research-2026-11-07.md`.

## Hasil ukur (PR #12, Nov 2026)

Diukur live pada head PR #12 (bukan estimasi). Angka yang sama tercantum di README § Performance (measured).

| Metric | Before | After |
|---|---|---|
| Refresh cycle (typical) | 7,214–8,278 ms | **264–461 ms** |
| Snapshot payload (gzip) | 35,487 B | **8,569 B (−76%)** |
| History endpoint | full table, unbounded | paginated (`?limit`, default 2000, max 10000) |
| SQLite | default journal | **WAL** + `busy_timeout=5000` + `synchronous=NORMAL` |
| LLM summaries | unbounded | capped: max **8** pending, **20**/hour |
| Service memory cap | none | **MemoryMax=1G** (systemd) |
| Server RSS (live) | — | ~29 MB at PR #12 deploy (~40 MB later as snapshots grow) |

## Log `[perf]`

Satu baris per siklus refresh (30 dtk) di stdout service:

```
[perf] refresh total=1234ms collect=1100ms sys=12ms docker=30ms ts=210ms git=340ms ps=45ms tmux=25ms omp_stats=0ms omp_usage=0ms sessions=180ms sources=900ms alert=2ms db=15ms
```

- `total` = seluruh `refresh()` (kolektor + evaluasi alert + tulis SQLite + broadcast).
- `collect` = `hq_collect::collect_snapshot` saja.
- `sys` = `/proc` + `df`; `docker` = `docker ps -a`; `ts` = `tailscale status`;
  `git` = `git log -1` per repo di `~/projects` (paralel per repo, B1); `ps` = `ps -eo`;
  `tmux` = `list-sessions` + `list-panes`.
- `omp_stats` / `omp_usage` = `omp stats --json` / `omp usage`; **0 = cache
  masih segar** (stats TTL 300 dtk, usage TTL 5 mnt) sehingga perintah tidak jalan.
- `sessions` = baca + parse file sesi OMP (maks 40 file, 24 jam terakhir).
- `sources` = seluruh loop registry sumber (omp, cursor, Muse, coding-agents, mcp).
- `alert` = evaluasi + rekonsiliasi alert; `db` = tulis `alert_state` + `snapshots`.

Satu baris per pekerjaan perapi LLM (utas latar, bukan siklus refresh):

```
[perf] llm task=<id> model=opencode-go/beam-free dur_ms=12345 ok=true
```

Cap B2 (dilog saat terjadi):

```
[llm] drop task=<id> pending=8 max_pending=8
[llm] skip task=<id> budget jam ini habis (max_per_hour=20)
```

- `drop` = antrean penuh saat enqueue (job dibuang, penghitung `dropped` naik).
- `skip` = anggaran panggilan jam ini habis (job dilepas dari pending).
- Env: `HQ_LLM_MAX_PENDING` (default 8), `HQ_LLM_MAX_PER_HOUR` (default 20).

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
  "perf": { "refresh_total_ms": 1234, "...": "..." },
  "llm": { "pending": 0, "calls_this_hour": 3, "max_per_hour": 20, "dropped": 0 },
  "refresh": { "count": 42, "last_at_ms": 1794015000000, "last_total_ms": 1234 }
}
```

- `ok` = liveness proses (handler jalan). `db.ok=false` + `error` bila query
  gagal atau mutex DB sibuk saat dicek (try_lock, tanpa blokir).
- `stocks.fresh` = ada simbol yang berhasil diambil hari ini (WIB).
  `age_secs` = detik sejak fetch terakhir; `null` bila belum pernah fetch.
- `mem` dari `/proc/self/status` (VmRSS / VmHWM). `disk.percent` dari snapshot
  terakhir — sumbernya `/mnt/c` (C: Windows) dengan fallback `/` (B6).
  `perf` = hasil ukur siklus refresh terakhir (sama seperti log).
  `llm` = antrean + anggaran LLM jam ini (B2); `refresh` = penghitung siklus (B6).
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

## Fase PERBAIKI (B1–B5)

Urutan sesuai §Kesimpulan laporan ukur (bukti, bukan asumsi):

1. **B1 — `refresh()` di `spawn_blocking`**: `collect_snapshot` (seluruh
   subprocess) jalan di blocking thread; `CollectState` dipindah keluar dari
   `RwLock` (`mem::take`) dan dikembalikan sesudahnya; bila blocking task
   panik, state dipulihkan kosong + siklus dilewati (snapshot lama tetap
   tampil). Tick memakai `MissedTickBehavior::Skip` (siklus tertinggal
   dilewati, bukan ditumpuk).
2. **B1-cache — kolektor lambat**: `collect_git` paralel per repo
   (`std::thread::scope`, urutan deterministik nama repo); TTL probe
   (tailscale + git) 30 dtk → 60 dtk; TTL `omp stats` 60 dtk → 300 dtk
   (kuota `omp usage` tetap 5 mnt).
3. **B2 — cap subprocess LLM**: antrean maks `HQ_LLM_MAX_PENDING` (default 8,
   drop + log `[llm] drop` + penghitung `dropped`); anggaran
   `HQ_LLM_MAX_PER_HOUR` (default 20, dicadangkan saat enqueue, log
   `[llm] skip` bila habis); terekspos di `/api/health` → `llm`.
4. **B3 — SQLite WAL**: `db_init` menjalankan
   `PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000; PRAGMA synchronous=NORMAL;`
   (baca tidak lagi memblokir tulis snapshot tiap siklus).
5. **B4 — `/api/history` paginasi (BUKAN indeks)**: `?limit=` (default 2000,
   maks 10000) + `?offset=` + `total` di respons; jendela = baris TERBARU
   (`ORDER BY at DESC LIMIT/OFFSET`, dibalik ke ASC di Rust). Bukan full scan
   (`at` = INTEGER PRIMARY KEY = rowid); biaya nyata adalah payload.
6. **B5 — kompresi HTTP**: `tower-http` fitur `compression-gzip` +
   `CompressionLayer` di router (mencakup aset statik ServeDir).
7. **B6 — health diperdalam + MemoryMax**: `disk.percent` dari `/mnt/c`
   (fallback `/`); `/api/health` tambah `llm` + `refresh`
   (`count`, `last_at_ms`, `last_total_ms`); unit systemd `MemoryMax=1G`
   (di repo + diterapkan saat deploy).

Catatan drift: unit systemd di repo ini memakai `HQ_INTERVAL=6`, sementara
unit live memakai `HQ_INTERVAL=30` — drift dicatat, TIDAK diubah di PR ini.
