# Adding a New Data Source / Connector

Rizal Agent HQ is designed to be **dynamic**: a new agent, tool, or service can
show up in the office without changing the UI, the data model, or the server.
There are **three paths**, from the most code-free to the deepest.

## Path 1 — Heartbeat API (no code, for any agent/tool)

An external agent just needs to check in periodically:

```bash
curl -X POST http://127.0.0.1:8900/api/heartbeat \
  -H 'Content-Type: application/json' \
  -d '{"agent":"Agent Name","role":"Its role","status":"bekerja","task":"What it is working on, as a human-friendly sentence"}'
```

The report is stored as a heartbeat file, and the agent shows up with a
*Working* status as long as its reports stay fresh (< 10 minutes). Good for:
bots, cron jobs, services on other machines, or agents that don't have a
dedicated collector yet.

## Path 2 — A new collector source (Rust, ±30 lines)

1. Write one function in `crates/hq-collect/src/lib.rs` (or a new file),
   following the pattern of the existing sources:

   ```rust
   fn src_example(_state: &mut CollectState, pr: &Probes) -> SourceOut {
       let mut out = SourceOut::default();
       // Read your source READ-ONLY and defensively:
       // a failure here must never take the snapshot down.
       out.agents.push(Agent {
           id: "example".into(),         // stable id, lowercase
           name: "Example".into(),
           emoji: "🧩".into(),
           role: "Short role description".into(),
           kind: AgentKind::Utama,        // Utama | Worker | Mcp
           parent: None,                  // parent id for workers
           status: Status::Bekerja,       // ONLY the 6 uniform statuses
           activity: "One human-friendly sentence about what it is working on".into(),
           provider: None, model: None,
           usage: Usage::default(),
           processes: 0, cpu_percent: 0.0, active_alerts: 0,
       });
       out
   }
   ```

2. Register **one line** in `registry()`:

   ```rust
   ("example", src_example),
   ```

3. Done. The new agent automatically appears in the 3D office, the cards, the
   detail view, alerts, and the daily report — because every panel reads the
   same uniform `Snapshot` model from `hq-core`.

Available to your source: `Probes` (classified system processes, tmux, docker,
git, system info — computed once and shared), `CollectState` (a cache across
cycles), and the `run_timeout`, `redact`, and `humanize` helpers.

**House rules for new sources:**
- Read-only. Never change machine state from a collector.
- Defensive: wrap anything that can fail; return whatever you have.
- Scrub: all free text must pass through `hq_core::redact::redact`.
- Human-friendly: `activity` is always ONE sentence a human can understand.
- Statuses are only the six `Status` values — no source-specific statuses.

## Path 3 — Turn sources on/off without recompiling

```bash
HQ_SOURCES=omp,cursor,Muse        # only these sources run
HQ_DISABLE_SOURCES=mcp,coding-agents # everything except these
```

Current source ids: `omp`, `cursor`, `Muse`, `coding-agents`, `mcp`
(see `known_sources()` / the registry for the latest).
