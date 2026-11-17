use hq_connectors::probes::Probes;
use hq_connectors::runtime::{
    Clock, CollectCtx, ProcessPresenceAdapter, RuntimeAdapter, SystemHealthEmitter,
};
use hq_core::{EventDraft, EventKind};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

struct WallClock;
impl Clock for WallClock {
    fn now_ms(&self) -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or_default()
    }
}

pub async fn run(args: &[String]) -> i32 {
    match args.first().map(String::as_str) {
        Some("collect") => collect_once(args).await,
        Some("serve") => serve(args).await,
        Some("device") => device(args).await,
        Some("openapi") => openapi(args),
        _ => {
            eprintln!("usage: rizal-agent-hq <collect|device|openapi|serve> [options]");
            2
        }
    }
}

/// Non-WebSocket/SSE v1 routes from 01-architecture §5 implemented by S06,
/// plus both health endpoints. WS, SSE and other legacy routes are exempt.
const EXPECTED_OPENAPI_PATHS: &[&str] = &[
    "/health",
    "/api/v1/health",
    "/api/v1/auth/pair",
    "/api/v1/auth/ws-ticket",
    "/api/v1/devices",
    "/api/v1/devices/{id}",
    "/api/v1/overview",
    "/api/v1/agents",
    "/api/v1/agents/{id}",
    "/api/v1/agents/{id}/tasks",
    "/api/v1/agents/{id}/logs",
    "/api/v1/agents/{id}/sessions",
    "/api/v1/agents/{id}/files",
    "/api/v1/agents/{id}/usage",
    "/api/v1/tasks",
    "/api/v1/tasks/{id}",
    "/api/v1/alerts",
    "/api/v1/alerts/{id}",
    "/api/v1/alerts/{id}/ack",
    "/api/v1/alerts/{id}/snooze",
    "/api/v1/alerts/{id}/resolve",
    "/api/v1/timeline",
    "/api/v1/events",
    "/api/v1/usage",
    "/api/v1/costs",
    "/api/v1/openapi.json",
];

fn check_openapi_paths(json: &serde_json::Value) -> Result<(), String> {
    let Some(paths) = json["paths"].as_object() else {
        return Err("no paths object in OpenAPI document".to_string());
    };
    if paths.is_empty() {
        return Err("no routes registered".to_string());
    }
    for path in EXPECTED_OPENAPI_PATHS {
        if !paths.contains_key(*path) {
            return Err(format!("missing required path: {path}"));
        }
    }
    Ok(())
}

fn openapi(args: &[String]) -> i32 {
    let mut out: Option<&str> = None;
    let mut check = false;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--out" if index + 1 < args.len() => {
                out = Some(args[index + 1].as_str());
                index += 2;
            }
            "--check" => {
                check = true;
                index += 1;
            }
            other => {
                eprintln!("unknown option: {other}");
                return 2;
            }
        }
    }
    let Some(out) = out else {
        eprintln!("usage: rizal-agent-hq openapi --out <path> [--check]");
        return 2;
    };
    use utoipa::OpenApi;
    let doc = hq_api::openapi::ApiDoc::openapi();
    let json = match serde_json::to_value(&doc) {
        Ok(json) => json,
        Err(error) => {
            eprintln!("cannot serialize openapi: {error}");
            return 1;
        }
    };
    if check {
        let rendered = json.to_string();
        let schemas = &json["components"]["schemas"];
        let mut start = 0;
        while let Some(index) = rendered[start..].find("#/components/schemas/") {
            let absolute = start + index + "#/components/schemas/".len();
            let name: String = rendered[absolute..]
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if schemas.get(&name).is_none() {
                eprintln!("missing schema: {name}");
                return 1;
            }
            start = absolute;
        }
        if let Err(error) = check_openapi_paths(&json) {
            eprintln!("{error}");
            return 1;
        }
    }
    match serde_json::to_string_pretty(&json)
        .ok()
        .and_then(|text| std::fs::write(out, text).ok())
    {
        Some(()) => 0,
        None => {
            eprintln!("cannot write {out}");
            1
        }
    }
}

async fn collect_once(args: &[String]) -> i32 {
    if args != ["collect", "--once", "--json"] {
        eprintln!("usage: rizal-agent-hq collect --once --json");
        return 2;
    }

    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"));
    let data_dir = data_dir_from(
        &home,
        std::env::var_os("HQ_DATA").as_deref(),
        std::env::var_os("XDG_DATA_HOME").as_deref(),
    );
    let probes = Probes::collect(&configured_repos(&home), &data_dir);
    let now_ms = WallClock.now_ms();
    let adapter = ProcessPresenceAdapter::default();
    let ctx = CollectCtx {
        now_ms,
        processes: probes.processes.clone(),
    };
    let mut events = adapter.observe(&ctx).await.unwrap_or_default();
    if let Some(health) = SystemHealthEmitter::new(WallClock).observe(&probes) {
        events.push(health);
    }

    let mut output = serde_json::to_value(&probes).unwrap_or_default();
    if let Some(object) = output.as_object_mut() {
        object.insert(
            "events".to_owned(),
            serde_json::Value::Array(events.iter().map(event_json).collect()),
        );
    }
    println!("{output}");
    if probes.all_failed() { 1 } else { 0 }
}

async fn device(args: &[String]) -> i32 {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"));
    let data_dir = data_dir_from(
        &home,
        std::env::var_os("HQ_DATA").as_deref(),
        std::env::var_os("XDG_DATA_HOME").as_deref(),
    );
    let Ok(store) = hq_store::Store::open(hq_store::StoreOptions::for_dir(&data_dir)).await else {
        eprintln!("cannot open data store");
        return 1;
    };
    match args.get(1).map(String::as_str) {
        Some("add") => {
            let mut name = "device".to_string();
            let mut scopes = vec!["read".to_string()];
            let mut index = 2;
            while index + 1 < args.len() {
                match args[index].as_str() {
                    "--name" => name = args[index + 1].clone(),
                    "--scope" => scopes = args[index + 1].split(',').map(str::to_string).collect(),
                    other => {
                        eprintln!("unknown option: {other}");
                        return 2;
                    }
                }
                index += 2;
            }
            let code = match hq_api::auth::random_code(8) {
                Ok(code) => code,
                Err(error) => {
                    eprintln!("cannot generate pairing code: {error}");
                    return 1;
                }
            };
            let now = hq_api::auth::now_ms();
            let record = hq_store::PairingRecord {
                name,
                scopes,
                created_ms: now,
                expires_ms: now + 600_000,
            };
            if store
                .create_pairing(&hq_api::auth::token_hash(&code), record)
                .await
                .is_err()
            {
                eprintln!("cannot store pairing code");
                return 1;
            }
            println!("Pairing code: {code}");
            0
        }
        Some("list") => match store.devices().await {
            Ok(devices) => {
                let views: Vec<hq_api::auth::DeviceView> =
                    devices.into_iter().map(Into::into).collect();
                println!(
                    "{}",
                    serde_json::to_string_pretty(&views).unwrap_or_default()
                );
                0
            }
            Err(_) => {
                eprintln!("cannot read devices");
                1
            }
        },
        _ => {
            eprintln!("usage: rizal-agent-hq device <add|list> [options]");
            2
        }
    }
}

fn event_json(event: &EventDraft) -> serde_json::Value {
    serde_json::json!({
        "event_ts_ms": event.event_ts_ms,
        "kind": if event.kind == EventKind::SystemHealthObserved { "system_health_observed" } else { "agent_observed" },
        "subject": event.subject.key(),
        "source": event.source.0,
        "payload": event.payload,
    })
}

fn configured_repos(home: &std::path::Path) -> Vec<String> {
    let config_dir = std::env::var_os("HQ_CONFIG")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .map(|path| path.join("rizal-agent-hq"))
        })
        .unwrap_or_else(|| home.join(".config").join("rizal-agent-hq"));
    std::fs::read_to_string(config_dir.join("config.toml"))
        .ok()
        .map(|contents| parse_repos_from_config(&contents))
        .unwrap_or_default()
}

fn parse_repos_from_config(contents: &str) -> Vec<String> {
    use std::str::FromStr;
    let Ok(document) = toml_edit::DocumentMut::from_str(contents) else {
        return Vec::new();
    };
    document
        .get("repos")
        .and_then(toml_edit::Item::as_array)
        .map(|repos| {
            repos
                .iter()
                .filter_map(toml_edit::Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn data_dir_from(
    home: &std::path::Path,
    hq_data: Option<&std::ffi::OsStr>,
    xdg_data_home: Option<&std::ffi::OsStr>,
) -> PathBuf {
    hq_data
        .map(PathBuf::from)
        .or_else(|| {
            xdg_data_home
                .map(PathBuf::from)
                .map(|path| path.join("rizal-agent-hq"))
        })
        .unwrap_or_else(|| home.join(".local/share/rizal-agent-hq"))
}

struct ServeArgs {
    data_dir: PathBuf,
    seed: Option<PathBuf>,
    allow_seed: bool,
    port: u16,
}

fn parse_serve_args(args: &[String]) -> Result<ServeArgs, String> {
    let mut data_dir = None;
    let mut seed = None;
    let mut allow_seed = false;
    let mut port = None;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--data" if index + 1 < args.len() => {
                data_dir = Some(PathBuf::from(&args[index + 1]));
                index += 2;
            }
            "--seed" if index + 1 < args.len() => {
                seed = Some(PathBuf::from(&args[index + 1]));
                index += 2;
            }
            "--port" if index + 1 < args.len() => {
                port = Some(
                    args[index + 1]
                        .parse::<u16>()
                        .map_err(|_| format!("invalid port: {}", args[index + 1]))?,
                );
                index += 2;
            }
            "--allow-seed" => {
                allow_seed = true;
                index += 1;
            }
            other => return Err(format!("unknown option: {other}")),
        }
    }
    Ok(ServeArgs {
        data_dir: data_dir.ok_or("--data is required")?,
        seed,
        allow_seed,
        port: port.ok_or("--port is required")?,
    })
}

fn parse_seed_fixture(value: &serde_json::Value) -> Result<Vec<EventDraft>, String> {
    hq_api::handlers::meta::parse_seed_events(value)
}

fn rebase_seed_timestamps(drafts: &mut [EventDraft], now_ms: i64) {
    let Some(max_ts) = drafts.iter().map(|draft| draft.event_ts_ms).max() else {
        return;
    };
    let shift = now_ms - 60_000 - max_ts;
    for draft in drafts {
        draft.event_ts_ms += shift;
    }
}

async fn load_seed_fixture(
    store: &hq_store::Store,
    path: &std::path::Path,
    now_ms: i64,
) -> Result<usize, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|error| format!("invalid fixture JSON: {error}"))?;
    let mut drafts = parse_seed_fixture(&value)?;
    rebase_seed_timestamps(&mut drafts, now_ms);
    let report = hq_store::Ingestor::new(store.clone())
        .ingest(drafts)
        .await
        .map_err(|error| format!("cannot ingest fixture: {error}"))?;
    Ok(report.accepted.len())
}

async fn serve(args: &[String]) -> i32 {
    let opts = match parse_serve_args(args) {
        Ok(opts) => opts,
        Err(error) => {
            eprintln!("{error}");
            eprintln!(
                "usage: rizal-agent-hq serve --data <dir> --port <port> [--seed <fixture.json>] [--allow-seed]"
            );
            return 2;
        }
    };
    if opts.allow_seed {
        // SAFETY: CLI setup happens before any request is served; the endpoint
        // reads the gate on every request rather than caching it at startup.
        unsafe { std::env::set_var(hq_api::handlers::meta::SEED_ENV, "1") };
    }
    if let Err(error) = std::fs::create_dir_all(&opts.data_dir) {
        eprintln!("cannot create data dir: {error}");
        return 1;
    }
    let Ok(store) = hq_store::Store::open(hq_store::StoreOptions::for_dir(&opts.data_dir)).await
    else {
        eprintln!("cannot open data store");
        return 1;
    };
    if let Some(path) = &opts.seed {
        match load_seed_fixture(&store, path, WallClock.now_ms()).await {
            Ok(accepted) => println!("seeded {accepted} events"),
            Err(error) => {
                eprintln!("cannot seed fixture: {error}");
                return 1;
            }
        }
    }

    let listener = match tokio::net::TcpListener::bind(("127.0.0.1", opts.port)).await {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("cannot bind 127.0.0.1:{}: {error}", opts.port);
            return 1;
        }
    };
    let state = hq_api::AppState {
        store,
        ring: Arc::new(Mutex::new(hq_store::PatchRing::new(1000))),
        auth: hq_api::auth::AuthConfig::default(),
        started_at: Instant::now(),
        tickets: Default::default(),
    };
    println!("serving on http://127.0.0.1:{}", opts.port);
    let app = hq_api::build_router(state).into_make_service_with_connect_info::<SocketAddr>();
    if let Err(error) = axum::serve(listener, app).await {
        eprintln!("server error: {error}");
        return 1;
    }
    0
}

#[cfg(test)]
mod tests {
    use super::parse_repos_from_config;

    #[test]
    fn generated_openapi_contains_all_required_paths() {
        let required = super::EXPECTED_OPENAPI_PATHS;
        let json =
            serde_json::to_value(<hq_api::openapi::ApiDoc as utoipa::OpenApi>::openapi()).unwrap();
        let paths = json["paths"].as_object().unwrap();
        for path in required {
            assert!(paths.contains_key(*path), "missing required path: {path}");
        }
    }

    #[test]
    fn openapi_check_rejects_document_missing_required_path() {
        let mut json =
            serde_json::to_value(<hq_api::openapi::ApiDoc as utoipa::OpenApi>::openapi()).unwrap();
        json["paths"]
            .as_object_mut()
            .unwrap()
            .remove("/api/v1/timeline");
        let error = super::check_openapi_paths(&json).unwrap_err();
        assert!(error.contains("/api/v1/timeline"), "{error}");
    }

    #[test]
    fn repos_default_to_empty_and_read_configured_paths() {
        assert!(parse_repos_from_config("").is_empty());
        assert_eq!(
            parse_repos_from_config("repos = [\"/work/alpha\", \"/work/beta\"]"),
            vec!["/work/alpha", "/work/beta"]
        );
    }

    #[test]
    fn data_path_uses_hq_override_then_xdg_then_default() {
        let home = std::path::Path::new("/home/test");
        assert_eq!(
            super::data_dir_from(home, Some(std::ffi::OsStr::new("/data/hq")), None),
            std::path::PathBuf::from("/data/hq")
        );
        assert_eq!(
            super::data_dir_from(home, None, Some(std::ffi::OsStr::new("/data"))),
            std::path::PathBuf::from("/data/rizal-agent-hq")
        );
        assert_eq!(
            super::data_dir_from(home, None, None),
            std::path::PathBuf::from("/home/test/.local/share/rizal-agent-hq")
        );
    }

    fn alert_draft(ts: i64, id: &str) -> serde_json::Value {
        serde_json::json!({
            "event_ts_ms": ts,
            "kind": "alert_raised",
            "subject": format!("alert:{id}"),
            "source": "test:e2e",
            "payload": {
                "id": id,
                "fingerprint": format!("fp-{id}"),
                "rule_id": "rule-1",
                "level": "warning",
                "message": id
            }
        })
    }

    #[test]
    fn seed_fixture_parser_accepts_bare_arrays() {
        let value = serde_json::json!([alert_draft(1_000, "a1"), alert_draft(2_000, "a2")]);
        let drafts = super::parse_seed_fixture(&value).expect("bare fixture parses");
        assert_eq!(drafts.len(), 2);
        assert_eq!(drafts[0].event_ts_ms, 1_000);
        assert_eq!(drafts[1].subject.key(), "alert:a2");
    }

    #[tokio::test]
    async fn seed_fixture_loader_rebases_max_timestamp_to_one_minute_ago_and_ingests() {
        let dir = tempfile::tempdir().unwrap();
        let store = hq_store::Store::open(hq_store::StoreOptions::for_dir(dir.path()))
            .await
            .unwrap();
        let fixture = dir.path().join("fixture.json");
        std::fs::write(
            &fixture,
            serde_json::to_vec(&serde_json::json!({
                "events": [alert_draft(1_000, "a1"), alert_draft(5_000, "a2")]
            }))
            .unwrap(),
        )
        .unwrap();

        let accepted = super::load_seed_fixture(&store, &fixture, 1_000_000)
            .await
            .expect("fixture loads");
        assert_eq!(accepted, 2);

        let events = store.read_since(hq_store::Seq(0), 10).await.unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].draft.event_ts_ms, 936_000);
        assert_eq!(events[1].draft.event_ts_ms, 940_000);
        assert_eq!(events[0].draft.payload["message"], "a1");
    }

    #[test]
    fn serve_flags_parse_data_seed_port_and_allow_seed() {
        let args = [
            "serve",
            "--data",
            "/tmp/hq-data",
            "--seed",
            "fixture.json",
            "--allow-seed",
            "--port",
            "8901",
        ]
        .map(String::from);
        let parsed = super::parse_serve_args(&args).expect("flags parse");
        assert_eq!(parsed.data_dir, std::path::PathBuf::from("/tmp/hq-data"));
        assert_eq!(parsed.seed, Some(std::path::PathBuf::from("fixture.json")));
        assert!(parsed.allow_seed);
        assert_eq!(parsed.port, 8901);
    }

    #[test]
    fn serve_requires_data_dir_and_port() {
        let args = ["serve", "--seed", "fixture.json"].map(String::from);
        assert!(super::parse_serve_args(&args).is_err());
    }
}
