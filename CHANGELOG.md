# Changelog

All notable changes to Rizal Agent HQ are documented here.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [2.5.0] - 2026-11-02

### Added
- Repository best practices: CI on every push/PR (rustfmt, Clippy, tests,
  release build, JS syntax), tag-triggered GitHub Releases with a prebuilt
  Linux binary, Dependabot, issue templates, LICENSE, CONTRIBUTING,
  SECURITY, and this changelog.

### Changed
- **Award-pass UI redesign**: flat minimalist header, one unified stage
  control pill (pause + camera reset + zones only), and a consistent
  stroke-style SVG icon system across all UI chrome (no more emoji icons).
- Agent chips and cards now use status dots and color monograms.

### Fixed
- **Camera persistence**: the camera no longer snaps back to the default
  view — pan/zoom targets survive mouse-up and page reloads
  (localStorage `hq-cam-v2`), hand control always cancels preset flights.
- Performance: render signature cache per panel, capped dock lists,
  chip DOM writes only on change, DPR ≤ 1.5, 1024px shadow maps,
  rendering pauses in hidden tabs.

## [2.4.0] - 2026-11-02

### Changed
- Layout rebuilt as a docked app-shell grid (single-row header, stage card,
  right rail, task/log dock below the stage) — nothing floats over the 3D
  canvas anymore; name chips resolve collisions and ellipsize.

## [2.3.0] - 2026-11-02

### Added
- LLM session-summary polisher ($0 models, SQLite cache, template fallback).
- Models & Providers dashboard: every model from every provider with usage
  and cash-vs-shadow cost classes.
- Sims-like stage controls and realtime per-agent process lines above heads.
- Trilingual UI (English / Bahasa Indonesia / 中文）.
- Parallel main sessions appear as their own figures (budget of 12).

### Fixed
- Alert acknowledge/snooze actions now persist in SQLite and broadcast
  instantly over WebSocket (they previously reset on restart).
- Muse's status is heartbeat-driven: <15 min = Working, <2 h = Thinking.
- Refresh interval reduced from 30 s to 6 s with split cache TTLs.

## [2.1.0] - 2026-11-02

### Added
- "Call a meeting": human agents walk to the meeting room and back.
- Task board (kanban) view, Facilities overlay, clickable interior hotspots.

## [2.0.0] - 2026-11-02

### Added
- First public release: Rust (Axum + WebSocket + SQLite) single binary,
  isometric Three.js office, per-agent monitoring for OMP (per-session
  workers), Cursor, Muse and MCP servers, humanized tasks & logs,
  per-agent usage with cash-vs-shadow costs, alert center with working
  ack/snooze, plug-and-play installer, daily report.

[2.5.0]: https://github.com/rzlco666/rizal-agent-hq/releases/tag/v2.5.0
[2.4.0]: https://github.com/rzlco666/rizal-agent-hq/releases/tag/v2.4.0
[2.3.0]: https://github.com/rzlco666/rizal-agent-hq/releases/tag/v2.3.0
[2.1.0]: https://github.com/rzlco666/rizal-agent-hq/releases/tag/v2.1.0
[2.0.0]: https://github.com/rzlco666/rizal-agent-hq/releases/tag/v2.0.0
