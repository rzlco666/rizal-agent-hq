PRAGMA journal_mode=WAL;
PRAGMA synchronous=NORMAL;
PRAGMA busy_timeout=5000;
PRAGMA foreign_keys=ON;

CREATE TABLE events (
  seq            INTEGER PRIMARY KEY AUTOINCREMENT,
  id             TEXT NOT NULL UNIQUE,
  event_ts_ms    INTEGER NOT NULL,
  ingest_ts_ms   INTEGER NOT NULL,
  kind           TEXT NOT NULL,
  subject        TEXT NOT NULL,
  source         TEXT NOT NULL,
  schema_version INTEGER NOT NULL,
  correlation_id TEXT,
  causation_id   TEXT,
  idempotency_key TEXT,
  payload        TEXT NOT NULL
);
CREATE INDEX events_ts      ON events(event_ts_ms);
CREATE INDEX events_subject ON events(subject, event_ts_ms);
CREATE INDEX events_kind    ON events(kind, event_ts_ms);
CREATE INDEX events_corr    ON events(correlation_id);
CREATE UNIQUE INDEX events_idem ON events(source, idempotency_key)
  WHERE idempotency_key IS NOT NULL;

CREATE TABLE agents (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  role TEXT NOT NULL,
  kind TEXT NOT NULL,
  parent_id TEXT,
  runtime_key TEXT NOT NULL UNIQUE,
  status TEXT NOT NULL,
  activity_json TEXT NOT NULL,
  provider TEXT,
  model TEXT,
  requests INTEGER NOT NULL DEFAULT 0,
  input_tokens INTEGER NOT NULL DEFAULT 0,
  output_tokens INTEGER NOT NULL DEFAULT 0,
  cache_tokens INTEGER NOT NULL DEFAULT 0,
  cash_usd REAL NOT NULL DEFAULT 0,
  shadow_usd REAL NOT NULL DEFAULT 0,
  processes INTEGER NOT NULL DEFAULT 0,
  cpu_percent REAL NOT NULL DEFAULT 0,
  evidence TEXT NOT NULL,
  last_seen_ms INTEGER NOT NULL,
  updated_seq INTEGER NOT NULL
);

CREATE TABLE sessions (
  id TEXT PRIMARY KEY,
  agent_id TEXT NOT NULL,
  runtime TEXT NOT NULL,
  cwd TEXT,
  started_ms INTEGER NOT NULL,
  ended_ms INTEGER,
  outcome TEXT,
  summary_json TEXT,
  requests INTEGER NOT NULL DEFAULT 0,
  input_tokens INTEGER NOT NULL DEFAULT 0,
  output_tokens INTEGER NOT NULL DEFAULT 0,
  cache_tokens INTEGER NOT NULL DEFAULT 0,
  cash_usd REAL NOT NULL DEFAULT 0,
  shadow_usd REAL NOT NULL DEFAULT 0,
  updated_seq INTEGER NOT NULL
);
CREATE INDEX sessions_agent ON sessions(agent_id, started_ms DESC);

CREATE TABLE tasks (
  id TEXT PRIMARY KEY,
  agent_id TEXT NOT NULL,
  session_id TEXT,
  title_json TEXT NOT NULL,
  phase TEXT NOT NULL,
  started_ms INTEGER NOT NULL,
  updated_ms INTEGER NOT NULL,
  ended_ms INTEGER,
  outcome_json TEXT,
  ordinal INTEGER NOT NULL DEFAULT 0,
  updated_seq INTEGER NOT NULL
);
CREATE INDEX tasks_agent ON tasks(agent_id, updated_ms DESC);
CREATE INDEX tasks_phase ON tasks(phase, updated_ms DESC);

CREATE TABLE task_steps (
  task_id TEXT NOT NULL,
  idx INTEGER NOT NULL,
  label_json TEXT NOT NULL,
  done INTEGER NOT NULL DEFAULT 0,
  at_ms INTEGER,
  PRIMARY KEY (task_id, idx)
);

CREATE TABLE logs (
  id TEXT PRIMARY KEY,
  agent_id TEXT NOT NULL,
  session_id TEXT,
  ts_ms INTEGER NOT NULL,
  level TEXT NOT NULL,
  sentence_json TEXT NOT NULL,
  raw TEXT,
  seq INTEGER NOT NULL
);
CREATE INDEX logs_agent ON logs(agent_id, ts_ms DESC);

CREATE VIRTUAL TABLE logs_fts USING fts5(
  log_id UNINDEXED, text, tokenize='unicode61 remove_diacritics 2'
);

CREATE TABLE alerts (
  id TEXT PRIMARY KEY,
  fingerprint TEXT NOT NULL UNIQUE,
  rule_id TEXT NOT NULL,
  level TEXT NOT NULL,
  state TEXT NOT NULL,
  subject TEXT NOT NULL,
  title_json TEXT NOT NULL,
  detail_json TEXT NOT NULL,
  opened_ms INTEGER NOT NULL,
  updated_ms INTEGER NOT NULL,
  ack_ms INTEGER,
  snooze_until_ms INTEGER,
  resolved_ms INTEGER,
  occurrences INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE alert_decisions (
  alert_id TEXT NOT NULL,
  actor TEXT NOT NULL,
  action TEXT NOT NULL,
  at_ms INTEGER NOT NULL,
  note TEXT
);

CREATE TABLE usage_minute (
  bucket_ms INTEGER NOT NULL,
  agent_id TEXT NOT NULL,
  provider TEXT NOT NULL,
  model TEXT NOT NULL,
  attribution TEXT NOT NULL,
  requests INTEGER NOT NULL,
  input_tokens INTEGER NOT NULL,
  output_tokens INTEGER NOT NULL,
  cache_tokens INTEGER NOT NULL,
  cash_usd REAL NOT NULL,
  shadow_usd REAL NOT NULL,
  PRIMARY KEY (bucket_ms, agent_id, provider, model, attribution)
);
CREATE TABLE usage_daily (
  day TEXT NOT NULL, agent_id TEXT NOT NULL, provider TEXT NOT NULL,
  model TEXT NOT NULL, attribution TEXT NOT NULL,
  requests INTEGER NOT NULL, input_tokens INTEGER NOT NULL,
  output_tokens INTEGER NOT NULL, cache_tokens INTEGER NOT NULL,
  cash_usd REAL NOT NULL, shadow_usd REAL NOT NULL,
  PRIMARY KEY (day, agent_id, provider, model, attribution)
);

CREATE TABLE system_health (
  ts_ms INTEGER PRIMARY KEY,
  cpu_percent REAL NOT NULL,
  mem_percent REAL NOT NULL,
  disk_percent REAL NOT NULL,
  uptime_secs INTEGER NOT NULL,
  docker_json TEXT,
  git_json TEXT,
  updated_seq INTEGER NOT NULL
);

CREATE TABLE connectors (
  id TEXT PRIMARY KEY, name TEXT NOT NULL, tier TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 0, health TEXT NOT NULL,
  last_ok_ms INTEGER, last_error TEXT, next_run_ms INTEGER,
  config_path TEXT, updated_seq INTEGER NOT NULL
);
CREATE TABLE connector_runs (
  id TEXT PRIMARY KEY, connector_id TEXT NOT NULL,
  started_ms INTEGER NOT NULL, finished_ms INTEGER,
  ok INTEGER, events INTEGER NOT NULL DEFAULT 0,
  duplicates INTEGER NOT NULL DEFAULT 0, error TEXT
);

CREATE TABLE approvals (
  id TEXT PRIMARY KEY, action_id TEXT NOT NULL,
  capability TEXT NOT NULL, target TEXT NOT NULL,
  input_json TEXT NOT NULL, state TEXT NOT NULL,
  requested_by TEXT NOT NULL, requested_ms INTEGER NOT NULL,
  expires_ms INTEGER NOT NULL, decided_by TEXT, decided_ms INTEGER,
  note TEXT
);

CREATE TABLE audits (
  id TEXT PRIMARY KEY, seq INTEGER, actor TEXT NOT NULL,
  capability TEXT NOT NULL, target TEXT NOT NULL,
  decision TEXT NOT NULL, at_ms INTEGER NOT NULL,
  correlation_id TEXT
);

CREATE TABLE knowledge_pages (
  id TEXT PRIMARY KEY, source TEXT NOT NULL, title TEXT NOT NULL,
  url TEXT, content_hash TEXT NOT NULL, word_count INTEGER NOT NULL,
  status TEXT NOT NULL, updated_ms INTEGER NOT NULL
);
CREATE TABLE knowledge_chunks (
  id TEXT PRIMARY KEY, page_id TEXT NOT NULL, idx INTEGER NOT NULL,
  text TEXT NOT NULL, embedding BLOB
);
CREATE VIRTUAL TABLE knowledge_fts USING fts5(
  chunk_id UNINDEXED, page_id UNINDEXED, text,
  tokenize='unicode61 remove_diacritics 2'
);

CREATE TABLE identity_aliases (alias TEXT PRIMARY KEY, canonical TEXT NOT NULL);
CREATE TABLE devices (
  id TEXT PRIMARY KEY, name TEXT NOT NULL, token_hash TEXT NOT NULL,
  scopes TEXT NOT NULL, created_ms INTEGER NOT NULL,
  expires_ms INTEGER, last_seen_ms INTEGER, revoked_ms INTEGER
);
CREATE TABLE telemetry_metrics (
  bucket_ms INTEGER NOT NULL,
  name TEXT NOT NULL,
  attrs_hash TEXT NOT NULL,
  attrs_json TEXT NOT NULL,
  value_sum REAL NOT NULL,
  count INTEGER NOT NULL,
  last_ms INTEGER NOT NULL,
  PRIMARY KEY (bucket_ms, name, attrs_hash)
);
CREATE TABLE telemetry_spans (
  trace_id TEXT NOT NULL,
  span_id TEXT NOT NULL,
  parent_span_id TEXT,
  name TEXT NOT NULL,
  service TEXT NOT NULL,
  start_ms INTEGER NOT NULL,
  duration_ms REAL NOT NULL,
  status TEXT NOT NULL,
  attrs_json TEXT,
  PRIMARY KEY (trace_id, span_id)
);
CREATE INDEX telemetry_spans_start ON telemetry_spans(start_ms);

CREATE TABLE summaries (
  key TEXT PRIMARY KEY,
  content_hash TEXT NOT NULL,
  text TEXT NOT NULL,
  created_ms INTEGER NOT NULL
);
CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE projection_checkpoint (name TEXT PRIMARY KEY, seq INTEGER NOT NULL);
CREATE TABLE outbox (
  id INTEGER PRIMARY KEY AUTOINCREMENT, seq INTEGER NOT NULL,
  kind TEXT NOT NULL, payload TEXT NOT NULL,
  attempts INTEGER NOT NULL DEFAULT 0, next_attempt_ms INTEGER NOT NULL
);
CREATE TABLE _migrations (version INTEGER PRIMARY KEY, name TEXT NOT NULL, checksum TEXT NOT NULL);

CREATE TABLE events_archive (
  seq            INTEGER PRIMARY KEY,
  id             TEXT NOT NULL UNIQUE,
  event_ts_ms    INTEGER NOT NULL,
  ingest_ts_ms   INTEGER NOT NULL,
  kind           TEXT NOT NULL,
  subject        TEXT NOT NULL,
  source         TEXT NOT NULL,
  schema_version INTEGER NOT NULL,
  correlation_id TEXT,
  causation_id   TEXT,
  idempotency_key TEXT,
  payload        TEXT NOT NULL
);