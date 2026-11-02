# PRD — Rizal Agent HQ v2

**Product:** Rizal Agent HQ — a 3D headquarters for monitoring *every agent* that works for Rizal.
**Total replacement for:** mason-agent-hq v1 (Node/JS) — the old repo is deleted and replaced by a new repo, `rzlco666/rizal-agent-hq`.
**Stack language:** Rust (backend + logic), web for the 3D display.
**Document status:** ✅ **FINAL — approved by Rizal on November 2, 2026** (discussed & decided section by section).
**Date:** November 2, 2026.

**Recap of final decisions:** K1-A (Three.js + Rust API) · 3D-C (mixed characters:
main agents are humans, workers are robots) · K2-C (hybrid humanization) · K3-C
(dashboard alerts + WhatsApp for Critical only) · K4-A (old repo deleted as soon
as v2 goes live) · K5 (port 8900). Section 1 (agent list & the uniform contract)
and Section 6 (per-agent usage) were accepted unchanged; the Section 1 defaults
apply (GitHub Actions = context; workers > 8 are grouped).

---

## 1. Why rebuild (v1 → v2 breakdown)

| # | Problem in v1 | v2 requirement (from Rizal) |
|---|---|---|
| 1 | Display **per machine** (WSL, Docker, GitHub presented as "residents" on par with agents) | Display **per AGENT**: what is shown is the agent itself (Muse, OMP, Cursor, etc.). Machines/WSL/Docker are demoted to *environment info*, not main characters |
| 2 | Office was a simple 2D canvas animation | **Good 3D design**: a real 3D office, a character per agent, rotatable/zoomable, click a character to focus |
| 3 | **Raw logs** (process arguments, `ps` text, etc.) — unpleasant to read | **Human-friendly** logs: plain human sentences ("OMP has started rewriting the payment module…"), raw logs only via a "view original" toggle |
| 4 | **Raw tasks** (process names, pids, shell commands) | **Human-friendly** tasks: a natural sentence title + sub-steps + progress + start time + estimated finish, like a team's task list |
| 5 | No way to get a **real detail view** per agent | A **per-agent detail** page/panel: summary, active & historical tasks, logs (human-friendly + raw), token/cost usage, sessions & changed files, action needed |
| 6 | No notification when something **needs Rizal** | **Alert center**: Muse/OMP/Cursor/WSL need action (login, approval, full quota, empty balance, error, death) → a prominent banner + alert list + handling status |
| 7 | Node/JS stack | **Rust**: one lightweight, fast binary that runs for months without memory leaks |
| 8 | The name "Mason" | His name is **Rizal** → `rizal-agent-hq`, titled "Rizal's Office" |

## 2. UNIFORM principles (apply to every agent, every panel, every option)

To keep things consistent, every agent is treated under the *same display contract*:

1. **Uniform statuses** — only these 6 statuses, for every agent:
   `Bekerja` · `Berpikir` · `Menunggu Rizal` · `Santai` · `Offline` · `Error`
   (status colors & icons are the same in the 3D office, cards, detail view, and alerts).
2. **Uniform agent card** — every agent always has: a 3D avatar, name, role,
   status, *one sentence* describing its current task in human terms, today's
   tokens & cost, and a badge with its active alert count.
3. **Uniform sentences** — logs & tasks are written in the same pattern:
   `[Agent] [human verb] [object] — [short detail]`.
   Example: "Cursor finished fixing 3 TypeScript errors in sample-next — moving on to running the tests."
4. **Uniform units** — tokens always in compact format (rb/jt/M), costs always
   split into `TUNAI` (cash) vs `bayangan` (shadow), times always WIB, dates
   always in Indonesian format.
5. **Uniform options** — every design decision in this PRD is presented in the
   same format: *Option — summary — pros — cons — recommendation*, so Rizal can
   decide consistently across all sections.
6. **One source of truth** — every panel reads the same agent data model from
   the Rust API; no panel does its own math.

## 3. Who the displayed "agents" are (per agent, not per machine)

| Agent | Displayed role | Source of activity evidence |
|---|---|---|
| **Muse** ✨ | Rizal's main assistant (on a separate VM) | Heartbeat API to the HQ server + Tailscale presence + active sessions |
| **OMP — Lead** 🧠 | The main coding engine on WSL | The `omp` process, OMP sessions & transcripts, `omp stats` |
| **OMP — Workers** (per subagent session) | Team members spawned by OMP Lead | OMP sub-sessions: model/provider, task, tokens, cost per worker |
| **Cursor** 🖱️ | Editor + agent worker | cursor-server, cursor-agent workers, `.cursor` file activity |
| **Claude Code / Codex / Devin** (when running) | Other detected coding agents | Each one's processes + session files |
| **MCP Servers** 🔌 | Tool servants (SQL, Figma, memory, etc.) | Active MCP processes + who their parent is |
| *(not agents)* WSL, Docker, Git, Tailscale | Demoted to the **Environment panel**: machine health, containers, latest commits | ps/docker/git — context support only |

Every newly detected agent automatically gets a desk in the 3D office and a
card under the uniform contract above — no manual configuration.

## 4. Main features

### 4.1 The 3D Office
- **Character style: ✅ DECIDED November 2, 2026 — Option 3D-C (Mixed).**
  Main agents (Muse, OMP Lead, Cursor, and other standalone coding agents) =
  **low-poly human** characters sitting at desks. OMP Workers (subagents) and
  MCP Servers = **small robots** in their parent's identity color — the
  difference between "permanent employees" and "workers on call" is obvious.
- 3D office (Three.js): floor, a desk per agent, an identity color per agent,
  a name plate on each desk.
- Animation follows the uniform statuses: typing while `Bekerja`, a thought
  cloud while `Berpikir`, a pulsing exclamation mark while `Menunggu Rizal`,
  💤 while `Santai`, dimmed while `Offline`, blinking red while `Error`.
- A speech balloon above each head containing *one human-friendly task sentence*
  describing what it is doing right now.
- Controls: rotate/zoom/pan the camera; camera preset buttons (Front, Top, Alert Focus).
- **Click a character/desk → the camera focuses + the agent detail panel opens** (see 4.2).
- Power-saving mode: when the tab is inactive or the device is weak, rendering
  is automatically reduced; a simple 2D mode is also available as a fallback.

### 4.2 Per-Agent Detail View (a real view)
A detail panel (right-hand drawer / `/agent/:id` page) with uniform tabs:
- **Summary** — status, current task (human-friendly), since when, progress,
  provider/model in use, this agent's active alerts.
- **Tasks** — running tasks + history of finished tasks (human-friendly titles,
  sub-steps auto-checked from activity evidence, duration, outcome).
- **Log** — human-friendly log (the default) with a **"View raw log"** switch;
  filterable by keyword and by level (info/activity/important).
- **Usage** — token in/out/cache & cost (TUNAI vs bayangan) per agent and per
  session/worker, mini charts for 24 hours & 7 days.
- **Sessions & Files** — active sessions, files this agent recently changed,
  commits it made, child processes (presented neatly, secrets still scrubbed).

### 4.3 Human-Friendly Tasks
- Every task is displayed as a sentence + context:
  *"Fixing the MySQL decode bug in strut-stack-sql"* — not `cargo test -p ...`.
- Produced by the **Humanization Engine** (§5): from commands/processes/
  sessions/commits into: a natural verb (writing, fixing, running tests,
  reading, waiting, …), an object (a recognized project/feature/file name),
  and step status.
- Tasks have a uniform lifecycle: `antre → berjalan → menunggu Rizal → selesai / gagal`
  (queued → running → waiting for Rizal → done / failed), complete with
  duration & final outcome (e.g. "done — 188 tests green").

### 4.4 Human-Friendly Logs
- The main log stream = human-friendly sentences in time order, grouped per agent.
- Transformation examples:
  - raw: `bun /home/rizal/.bun/bin/omp --model openai-codex/gpt-6.1-sol …`
  - human-friendly: "OMP started a new task using GPT (Codex)."
  - raw: `git commit ebf8117` → "OMP completed a milestone: commit 'release 1.1.0' created."
- Raw logs are still stored & can be opened per line (toggle), for auditing.

### 4.5 Alert Center — "Needs Rizal" 🚨
Alerts are created automatically by the **Alert Rule Engine** (§5). Uniform types:
| Type | Example trigger | Level |
|---|---|---|
| Needs login/access | Cursor asks for a fresh login; the `gh` token expired; Muse disconnected from Tailscale | Needs Action |
| Needs a decision | An agent stopped at an approval point; OMP is waiting for instructions in tmux | Needs Action |
| Quota full/nearly | An OMP provider ≥90% (warning) / 100% (critical) — e.g. Cursor & Antigravity Claude today | Warning → Critical |
| Balance running low | Estimated DeepSeek balance < $1 | Critical |
| Agent error/death | An agent process vanishes suddenly, repeated failed exit codes, a rising error rate | Critical |
| Environment | WSL offline/Tailscale disconnected, disk ≥90%, RAM ≥95%, an important container died | Warning |

- Display: a red/yellow banner at the top of the dashboard + a count badge in
  the title + an Alert Center list: who, what is needed, since when, and
  **"I've handled this"** and **"Snooze 1 hour"** buttons.
- Agents that are `Menunggu Rizal` also pulse in the 3D office (§4.1) — so
  Rizal only has to glance at the office to know someone is waiting.

### 4.6 Usage & Cost (per agent)
- Usage attribution **per agent & per worker/session** (not just OMP-global):
  token in/out/cache, requests, TUNAI vs bayangan cost, quota per provider.
- Daily/weekly summaries + charts; the estimated DeepSeek balance is always shown.
- The automatic daily report stays (Markdown, 23:55 WIB) — now per agent.

### 4.7 Environment (supporting, not the main character)
A small panel: WSL health (CPU/RAM/disk/uptime), Docker, latest git commits,
Tailscale status. Also appears as context in the related agent's detail view.

## 5. Rust architecture

```
rizal-agent-hq/  (Cargo workspace, one final binary: rizal-agent-hq)
├─ crates/
│  ├─ hq-core      the uniform model: Agent, Status, Task, LogEvent, Alert, Usage
│  │               + the Humanization Engine (sentence rules, §5.1)
│  ├─ hq-collect   collectors per SOURCE (not per machine in the UI):
│  │               omp (stats/usage/sessions), cursor, claude-heartbeat,
│  │               processes & tmux, git, docker, system — all read-only,
│  │               all text passes through secret redaction
│  ├─ hq-alert     the Alert Rule Engine (§4.5): evaluates every cycle,
│  │               dedup, levels, handling status
│  └─ hq-server    Axum: REST + WebSocket (live push), serves the static UI,
│                  SQLite storage (history, logs, alerts, daily snapshots)
├─ web/            UI: 3D office (Three.js) + panels — static files, no complex build
├─ systemd/        daily report service + timer (same pattern as v1)
└─ scripts/        install-wsl.sh, start/stop/status, v1 migration & removal
```

Technical principles:
- **One Rust binary** (target RSS < 60 MB), no Node/npm at runtime.
- **Dynamic (source registry)**: collectors are functions registered in
  `registry()` in hq-collect (built in: omp, cursor, Muse, coding-agents, mcp);
  a new source = one function + one registration line; on/off via the
  `HQ_SOURCES`/`HQ_DISABLE_SOURCES` env vars; external agents appear via the
  Heartbeat API with no code. Guide: docs/ADDING-A-SOURCE.md.
- **Plug & play**: `./scripts/install.sh` builds, installs to ~/rizal-agent-hq,
  and starts the systemd user service; a system without OMP/Cursor still runs
  (absent agents show as Offline).
- Defensive collectors: a failed/timed-out source → the agent is marked from
  its last evidence, and the dashboard never hangs with it (OMP stats stay
  cached because they are expensive).
- Live without manual refresh: WebSocket pushes status/task/alert changes.
- Read-only & safe: no write actions into other agents' systems; secrets
  (API keys/tokens in process arguments) are always scrubbed before display/storage.
- Always on: systemd user service (Restart=always + linger), SQLite data in
  `~/rizal-agent-hq/data/`, default port **8900** (new, to avoid clashing with v1 leftovers).

### 5.1 The Humanization Engine (logs & tasks become human-friendly)
Transforms raw evidence → sentences, in layers:
1. **Fact extraction**: agent, raw action (command/tool), object (project/file/feature
   from paths & history), outcome (exit/success/failure, test count, commit hash).
2. **Verb mapping**: a rule table (test → "menjalankan tes" ["running tests"],
   build → "membangun" ["building"], edit file → "menulis/memperbaiki"
   ["writing/fixing"], git commit → "menyelesaikan satu tahap" ["completing a
   milestone"], idle → "santai" ["idle"], etc.).
3. **Sentence assembly** in the uniform §2.3 pattern + time context ("baru saja"
   ["just now"], "sejak 14:02" ["since 14:02"]).
4. *(optional, see the §6 options)* **LLM polish**: long session summaries are
   condensed by a free model via OMP — for summaries only, not per-line logs.

## 6. Decisions for Rizal (uniform format — choose by number)

**K1 — 3D UI framework: ✅ DECIDED November 2, 2026 — Option A.**
- A. **Three.js (plain JS) + Rust API** — CHOSEN by Rizal. Mature 3D, fast to build, Rust stays the brain (API, collectors, humanization, alerts).
- B. Full Rust WASM (Leptos/Yew + WebGL) — not chosen (3D would be slower & riskier).
- C. Svelte + Three.js — not chosen (adds a Node build chain).

**Section 1 note (defaults that apply unless Rizal changes them):**
GitHub Actions has the status of *environment context* (not an agent). OMP
Workers appear as separate residents, with automatic grouping ("+N other
workers") when the number of active workers is > 8 so the office doesn't get
crowded.

**K2 — Humanization Engine: ✅ DECIDED November 2, 2026 — Option C (Hybrid).**
- Local rules for every per-line log/task (deterministic, offline, $0);
  a free LLM model via OMP *only* for long session summaries. When the LLM
  path is unavailable (full quota, etc.), summaries fall back to the rule
  template — the main log never dies with it. (Option A rules-only & B LLM-only
  were not chosen.)

**K3 — Alert channels: ✅ DECIDED November 2, 2026 — Option C.**
- Every level appears on the dashboard (banner + badge + Alert Center).
- At the **Critical** level only, alerts are also forwarded to **Rizal's WhatsApp**.
  Bridge mechanism: the HQ server flags critical alerts in its API; Muse
  (which is connected to WhatsApp & Tailscale) checks periodically (every
  ±15 minutes) and forwards new ones to WhatsApp — dedup follows the alert
  status in HQ, and it stops automatically once an alert is handled/recovered.
  (Options A & B were not chosen.)

**K4 — Fate of the v1 repo & service: ✅ DECIDED November 2, 2026 — Option A.**
The `mason-agent-hq` repo is **permanently deleted** as soon as v2 goes live;
the v1 service on WSL is removed at stage M5. (Option B archiving was not chosen.)

**K5 — Port & address: ✅ default accepted.** `127.0.0.1:8900`
(from Windows: http://localhost:8900).

## 7. Build stages (after the PRD is approved)

| Stage | Contents | Done when |
|---|---|---|
| M1 | Rust workspace skeleton + uniform model + collectors (OMP/Cursor/processes/tmux/git/docker/system) + SQLite | `cargo test` green; the `/api/agents` API returns real agents from Rizal's WSL |
| M2 | Humanization Engine + human-friendly tasks/logs + raw toggle | Logs & tasks render as sentences; the §4.4 transformation examples are proven on real data |
| M3 | 3D office + agent cards + per-agent detail view (5 tabs) | Click a character → detail opens; animation status = real status |
| M4 | Alert Engine + Alert Center + live WebSocket | A test alert (Cursor's currently-full quota) appears & can be marked handled |
| M5 | Install on WSL (systemd, port 8900), daily report, delete/archive v1 per K4, README | Dashboard live from Rizal's machine data; v1 service removed; old repo per the decision |

## 8. Out of scope (v2.0)
- Controlling agents from the dashboard (stop/start agent buttons) — v2 only *monitors & notifies*; actions remain Rizal's to take where the work actually happens.
- Multi-machine beyond Rizal's WSL + Muse's VM.
- Multi-user accounts/login — a local dashboard for Rizal alone.

## 9. Risks & mitigations
- *Heavy OMP stats/usage (±15–60 s)* → 5-minute cache + async collectors; agent status never waits on OMP.
- *Cursor/OMP sessions don't always have tidy transcript files* → layered evidence sources (processes → file activity → heartbeat), with status marked by its confidence level.
- *Heavy 3D on the laptop* → automatic power-saving mode + 2D fallback (K1 stays A).
- *Sensitive data* → secret redaction in collectors (the v1 pattern is kept & tested).
