# Contributing to Rizal Agent HQ

Thanks for your interest! Contributions of all sizes are welcome — a new data
source, a UI improvement, a bug fix, or a typo in the docs.

## Quick start

```bash
git clone https://github.com/rzlco666/rizal-agent-hq
cd rizal-agent-hq
cargo build --release -p hq-server
HQ_PORT=8900 ./target/release/rizal-agent-hq   # serves ./web by default
```

Prerequisites: a stable Rust toolchain (https://rustup.rs). No Node, no build
step for the frontend — `web/` is plain HTML/CSS/JS + Three.js.

Useful environment variables for development:

| Variable | Purpose |
|---|---|
| `HQ_PORT` | Port to bind (default `8900`, localhost only) |
| `HQ_WEB` | Path to the web assets folder (default `./web`) |
| `HQ_DATA` | Path to the SQLite data folder |
| `HQ_SOURCES` / `HQ_DISABLE_SOURCES` | Enable/disable data sources |

## Before you open a pull request

CI runs on every push and pull request, and it must stay green:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

For frontend changes, make sure the JS still parses (it is ES modules):

```bash
node --check web/app.js        # or copy to .mjs first on older Node
node --check web/office3d.js
```

## Adding a data source

HQ is built to be extended: collectors are registered sources, so a new agent
or tool (another coding assistant, a CI system, a chat bot…) can appear in the
office without touching the core. See
[docs/ADDING-A-SOURCE.md](docs/ADDING-A-SOURCE.md) for the walkthrough.

## Guidelines

- **Per agent, not per machine.** New UI should present *who* is working,
  human-first: friendly names, sentence-style activity ("X is reviewing Y"),
  costs always labelled **cash vs. shadow/estimated**.
- Keep the six agent statuses: `Working · Thinking · Waiting for you · Idle ·
  Offline · Error`.
- No heavy dependencies without discussion — the deploy target is a single
  ~5 MB static binary.
- Write commit messages that explain *why*, in the imperative mood.

## Reporting bugs & ideas

Use the issue templates. For anything security-sensitive, please follow
[SECURITY.md](SECURITY.md) instead of opening a public issue.
