<p align="center">
  <img src="web/logo.svg" width="128" alt="Rizal Agent HQ logo">
</p>

<h1 align="center">Rizal Agent HQ</h1>

<p align="center">
  <b>A 3D headquarters for every AI agent that works for you.</b><br>
  One room, every agent visibly at work — with <i>human-friendly</i> tasks &amp; logs,<br>
  per-agent usage, and alerts whenever someone <i>needs you</i>.
</p>

<p align="center">
  <img alt="Rust" src="https://img.shields.io/badge/Rust-dea584?logo=rust&logoColor=white">
  <img alt="Axum" src="https://img.shields.io/badge/Axum-0.8-2f3b52">
  <img alt="Three.js" src="https://img.shields.io/badge/Three.js-3D-049ef4?logo=threedotjs&logoColor=white">
  <img alt="SQLite" src="https://img.shields.io/badge/SQLite-003B57?logo=sqlite&logoColor=white">
  <img alt="MIT License" src="https://img.shields.io/badge/License-MIT-3ddc97">
  <img alt="Single binary" src="https://img.shields.io/badge/Deploy-1%20binary%20~9MB-ffb454">
</p>

<p align="center">
  <a href="https://github.com/rzlco666/rizal-agent-hq/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/rzlco666/rizal-agent-hq/actions/workflows/ci.yml/badge.svg"></a>
  <a href="https://github.com/rzlco666/rizal-agent-hq/releases"><img alt="Latest release" src="https://img.shields.io/github/v/release/rzlco666/rizal-agent-hq"></a>
  <a href="https://github.com/rzlco666/rizal-agent-hq/stargazers"><img alt="GitHub stars" src="https://img.shields.io/github/stars/rzlco666/rizal-agent-hq?style=social"></a>
</p>
<p align="center"><i>If HQ helps you see your agents, a ⭐ helps other agent-wranglers find it.</i></p>

---

<p align="center">
  <img src="docs/screenshot.png" width="880" alt="Rizal Agent HQ v2.5 running on real data: minimalist header, isometric 3D office with live agent chips, task & log dock below, and quota panels on the right">
</p>
<p align="center"><i>The 3D office running on real data — OMP, Cursor, and Muse in the front row; robot workers &amp; MCP behind them; Critical alerts and provider quotas in the right-hand panel.</i></p>

> **Latest release: v2.5.0** — [release notes](https://github.com/rzlco666/rizal-agent-hq/releases/tag/v2.5.0) (2026-11-02). Since then on `main`: live stock prices, Notion knowledge search, and the B1–B5 performance work — see [What's new](#-whats-new).

## ✨ Why does this project exist?

Modern AI agents no longer work alone: there's a main assistant, a multi-provider
coding engine, subagents spawned on demand, an AI editor, MCP servers — all
running at the same time, and their owner **can't see any of it**. Terminals are
full of raw logs, token costs are scattered everywhere, and nobody tells you
when an agent has been silently waiting for your decision for two hours.

**Rizal Agent HQ** turns all of that into one **living 3D office**:

| | |
|---|---|
| 🏢 **Per agent, not per machine** | What you see is *who is working*: every agent & worker has its own desk, character, and status. Machines (WSL/Docker/Git) are just supporting panels. |
| 🧑‍🤝‍🧑 **Mixed characters** | Main agents = low-poly humans; workers & MCP = small robots in their parent's colors. One glance tells you who's a permanent employee and who's an on-call contractor. |
| 🗣️ **Human-friendly logs & tasks** | Not `bash -c CARGO_BUILD_JOBS=2 cargo test …`, but *"OMP is running the strut-stack-sql test suite — 188 tests green."* The raw log is still there, behind a toggle. |
| 👆 **A real detail view** | Click a character → a 5-tab detail view: Summary, Tasks, Log, Usage, Sessions & Files. |
| 🚨 **"Needs You" alerts** | A full quota, a dwindling balance, an agent dying unexpectedly, a session waiting for input — all surface as tiered alerts in the Alert Center. |
| 💸 **Honest per-agent costs** | Tokens & costs are attributed per agent/session, always split into **CASH vs shadow** (subscription). No more unexplained "total $16" that belongs to no one. |
| 🔌 **Dynamic & plug-and-play** | Data sources live in a registry: adding a new connector takes one function + one line. External agents can join the office with a simple heartbeat POST. |

## 🆕 What's new

- **Live stocks (Nov 2026, PR #6–#8)** — `GET /api/stocks` serves live prices from EODHD (primary) with FMP as the backup for non-`.JK` symbols; the Stocks panel shows your watchlist from `stocks.json` plus a portfolio table with explicit cost basis. Watch-only: HQ never buys, sells, or advises.
- **Notion knowledge (Nov 2026, PR #10–#11)** — selected Notion pages sync into a local mirror (`notion_docs`, separate from trusted memory) with FTS5 BM25 search at `GET /api/knowledge/search`. Sync acceptance, measured live: **219/219 pages found (A1)** and **precision@5 0.90 (A5)**. Pages that trip the secret gate are quarantined, never stored redacted.
- **Performance B1–B5 (Nov 2026, PR #12)** — snapshot collection moved off the async runtime, LLM calls capped, SQLite on WAL, history paginated, gzip on. Typical refresh cycle **264–461 ms** (was 7,214–8,278 ms); gzip cuts a 35,487 B snapshot to 8,569 B (−76%). See [Performance](#-performance-measured).
- **In review: PR #13** — knowledge ranking fixes (R3+N2) plus the heartbeat-dir fix (F1). Not merged yet.

## ✨ Features

- **3D office** — isometric virtual office (Three.js): agent figures per session, zone chips with live counts, meeting room, task-board kanban, facilities panels, Sims-style controls (pause, speed, day/night, follow-cam), trilingual UI (EN/ID/中文).
- **Knowledge** — Notion sync (30-minute incremental + checkpointed backfill), FTS5 BM25 search, per-source status, quarantine for secret-like content, promote-to-memory flow (human-approved only — sync never writes trusted memory).
- **Stocks & portfolio** — live prices (EODHD primary, FMP backup), portfolio from `stocks.json` (positions + explicit `avg_buy_price`; P&L is "not available" until you fill positions — HQ never invents them, never advises).
- **Performance workspace** — per-agent progress donut, task counts, searchable task table, agent summary across all agents.
- **Alert Center** — tiered alerts with SQLite-backed acknowledge/snooze applied instantly over WebSocket.
- **Models & Providers** — per-model requests/tokens/cost, CASH vs SHADOW classes, LLM session-summary polisher ($0 models first, template fallback, SQLite cache).
- **Dynamic sources** — `omp`, `cursor`, `Muse`, `coding-agents`, `mcp`; any external agent joins via `POST /api/heartbeat`.

## 🚀 Install (plug & play)

You need: Linux with `systemd --user`, Rust (to build), and that's it.
OMP/Cursor/other agents are **optional** — whatever isn't there shows as
*Offline*, and everything else keeps running.

```bash
git clone https://github.com/rzlco666/rizal-agent-hq.git
cd rizal-agent-hq
./scripts/install.sh
```

Done. Open **http://localhost:8900/** — your 3D office fills up straight from
your own machine. Remove it any time with `./scripts/uninstall.sh`.

Run it manually, without systemd:

```bash
cargo run --release -p hq-server      # http://127.0.0.1:8900/
cargo test --workspace                # full suite (unit + integration)
```

Or foreground, no install at all:

```bash
./scripts/quickstart.sh               # default http://localhost:8900/, Ctrl+C to stop
```

## ⚙️ Configuration (all via env, no mandatory files)

Secrets live in the environment (or a systemd drop-in) — never in the repo.

| Env | Default | Meaning |
|---|---|---|
| `HQ_PORT` | `8900` | Server port |
| `HQ_BIND` | `127.0.0.1` | Listen address (local-first; bind `0.0.0.0` only behind a reverse proxy you trust) |
| `HQ_INTERVAL` | `6` | Snapshot collection interval (seconds) |
| `HQ_DATA` | `data` | Folder for SQLite + heartbeats |
| `HQ_WEB` | `web` | Static UI folder |
| `HQ_SOURCES` | *(all)* | Source allowlist, e.g. `omp,cursor,Muse` |
| `HQ_DISABLE_SOURCES` | — | Disable specific sources, e.g. `mcp` |
| `HQ_LLM` | `on` | LLM session-summary polisher on/off |
| `HQ_LLM_MODELS` | — | Model chain override ($0 models first by default) |
| `HQ_LLM_MAX_PENDING` | `8` | Max queued LLM summary tasks (B2 cap) |
| `HQ_LLM_MAX_PER_HOUR` | `20` | Max LLM summary calls per hour (B2 cap) |
| `HQ_DEST` | `$HOME/rizal-agent-hq` | Install target (only used by `scripts/install.sh`) |
| `NOTION_API_KEY` | — | Notion integration token (knowledge sync; without it the module runs degraded) |
| `EODHD_API_KEY` | — | EODHD token (live stock prices, primary) |
| `FMP_API_KEY` | — | FMP token (backup prices, non-`.JK` only) |

Full list with comments: [`.env.example`](.env.example).

## 🧩 Dynamic: adding new agents & connectors

Three paths, as deep as you need — the full guide is in
**[docs/ADDING-A-SOURCE.md](docs/ADDING-A-SOURCE.md)**:

1. **Heartbeat API (no code)** — any agent/tool POSTs to `/api/heartbeat`
   and immediately appears in the office.
2. **Collector source (±30 lines of Rust)** — one function + one registration
   line in `registry()`; the UI, alerts, and reports follow automatically
   thanks to the uniform model.
3. **On/off via env** — `HQ_SOURCES` / `HQ_DISABLE_SOURCES`, no recompiling.

Built-in sources: `omp` (stats, quotas, sessions & per-session workers),
`cursor`, `Muse` (heartbeat + Tailscale), `coding-agents` (Claude Code/Codex/
Devin when running), `mcp`.

## 🏗️ Architecture

```
            ┌─────────────────────── ONE RUST BINARY ────────────────────────┐
 sources ──▶│ hq-collect (registry) ─▶ hq-core (uniform model + Humanizer)   │
 (omp,      │        │                                  │                     │
  cursor,   │        ▼                                  ▼                     │
  Muse…)   │   hq-alert (rule engine)           hq-server (Axum)             │
            │        │                       REST + WebSocket + SQLite        │
            └────────┼───────────────────────────────────┬───────────────────┘
                     ▼                                   ▼
              Alert Center                       web/ — 3D office (Three.js)
              + ack/snooze (SQLite)              humans & robots, live via WS
```

Four crates (`hq-core`, `hq-collect`, `hq-alert`, `hq-server`), one static
binary (~9 MB release build, LTO + stripped). Knowledge (`notion_knowledge`)
and stocks live in `hq-server` as modules, behind the same snapshot model.

Non-negotiable principles:

- **6 uniform statuses** for every agent: `Bekerja · Berpikir · Menunggu Rizal · Santai · Offline · Error`.
- **Human-friendly first** — one fixed sentence pattern: `[Agent] [human verb] [object] — [detail]`.
- **Read-only & safe** — collectors never change anything on the machine; all free text is scrubbed of secrets (API keys/tokens are never displayed or stored).
- **One source of truth** — every panel reads the same `Snapshot` from `hq-core`.
- **Trusted memory is human-written only** — the Notion sync mirrors into `notion_docs` and never writes trusted memory; promotion is a human-approved action.

The full product specification: **[PRD.md](PRD.md)**.

## 🔌 API

Base: `http://127.0.0.1:8900`. JSON everywhere; live data only, no sample rows.

| Endpoint | What it does |
|---|---|
| `GET /api/health` | Liveness + deep status: uptime, DB, stocks freshness, memory, disk, LLM caps, last refresh |
| `GET /api/state` | Full snapshot (agents, tasks, alerts, quotas) |
| `GET /api/agents` | Agent list |
| `GET /api/alerts` | Alert list; `POST /api/alerts/:id/ack` and `/snooze` persist the action |
| `GET /api/history?limit=&offset=` | History rows, paginated (default limit 2000, max 10000) + `total` |
| `GET /api/stocks` | Watchlist + portfolio (live prices); `POST /api/stocks/refresh` fetches now, respecting the daily gate |
| `GET /api/portfolio` | Portfolio positions + cost basis |
| `GET /api/knowledge/search?q=` | FTS5 BM25 search over synced Notion pages |
| `GET /api/knowledge/pages` · `GET /api/knowledge/page/{id}` | Synced page list + one page |
| `GET /api/knowledge/sync/status` | Sync status per source (counts, last error, quarantine) |
| `POST /api/knowledge/sync` | Trigger an incremental sync now |
| `POST /api/knowledge/promote` | Promote a synced page toward trusted memory (human-approved) |
| `POST /api/heartbeat` | External agent heartbeat — joins the office |
| `GET /ws` | WebSocket: live snapshot + alert broadcasts |

## 📈 Performance (measured)

Measured live on the PR #12 head (Nov 2026), not estimated. Method and logs:
[docs/PERF.md](docs/PERF.md).

| Metric | Before | After |
|---|---|---|
| Refresh cycle (typical) | 7,214–8,278 ms | **264–461 ms** |
| Snapshot payload (gzip) | 35,487 B | **8,569 B (−76%)** |
| History endpoint | full table, unbounded | paginated (`?limit`, default 2000, max 10000) |
| SQLite | default journal | **WAL** + `busy_timeout=5000` + `synchronous=NORMAL` |
| LLM summaries | unbounded | capped: max **8** pending, **20**/hour |
| Service memory cap | none | **MemoryMax=1G** (systemd) |
| Server RSS (live) | — | ~29 MB |

## ⚠️ Honest limitations

- **Local-first.** HQ binds `127.0.0.1` by default and is meant to run on your own machine. Remote access needs a reverse proxy / tunnel you operate yourself.
- **Stocks are watch-only.** Prices come from EODHD (primary) and FMP (backup, non-`.JK` only); quotes are empty on weekends until the next fetch, and HQ never buys, sells, or gives investment advice. P&L stays "not available" until you fill positions yourself.
- **Knowledge search is good, not perfect.** Sync acceptance is A1 219/219 pages found and A5 precision@5 0.90 (one probe ranked #8, outside the top 5). Quarantined pages stay quarantined until a human reviews them.
- **LLM summaries are capped and cached.** At most 8 pending tasks and 20 calls/hour; when the budget is spent, summaries fall back to templates.
- **Alerts surface in HQ.** The Alert Center is live; forwarding to an external channel (Telegram/WhatsApp) is not wired up in this release.
- **One machine, one SQLite file.** No multi-user auth beyond the local bind; backups must include the `-wal`/`-shm` files (or checkpoint first).

## 🗺️ Status

| Stage | Contents | Status |
|---|---|---|
| M1 | Rust skeleton + collectors + SQLite + API | ✅ done, verified against real data |
| M2 | Session humanization + per-session workers + WebSocket | ✅ done, verified against real data |
| M3 | 3D office + 5-tab detail view | ✅ done, verified with screenshots of real data |
| M4 | Alert Center + ack/snooze | ✅ Alert Center live; actions persist in SQLite and apply instantly over WebSocket |
| M5 | GitHub release + systemd install + daily report | ✅ live on WSL (port 8900) + public repo; release v2.5.0 |
| M6 | Bright isometric virtual office redesign | ✅ isometric orthographic camera, furnished glass rooms per team, living environment, floating name chips, bright pill UI + agent search |
| M7 | Complete virtual office features | ✅ **Call Meeting**, **Task Board** 4-phase kanban, **Facilities**, clickable interior, version label v2.1 |
| M8 | Intelligence & control release | ✅ **LLM session-summary polisher**, **Models & Providers dashboard**, **alert actions that persist**, version v2.2 |
| M9 | Realtime Sims-style office | ✅ 6-second refresh, **every parallel OMP main session gets its own figure**, **The Sims controls**, **trilingual UI (EN/ID/中文)**, version v2.3 |
| M10 | Award-grade layout redesign | ✅ **Docked app-shell grid**, **agent name chips resolve collisions**, version v2.4 |
| M11 | Minimalist polish, honest camera, performance | ✅ **Flat minimalist header**, **stroke SVG icon system**, **one functional stage control bar**, **camera respects the user**, **render-on-change panels**, version v2.5 |

**Next:** PR #13 — knowledge ranking fixes (R3+N2) + heartbeat-dir fix (F1), in review.

## 🤝 Contributing

New data sources are the most valuable contribution: if your favorite
agent/tool doesn't have a collector yet, follow
[docs/ADDING-A-SOURCE.md](docs/ADDING-A-SOURCE.md) — one function, one
registration line, done.

## 📄 License

MIT © 2026 Syahrizal Hanif ([@rzlco666](https://github.com/rzlco666)) — built from scratch.
