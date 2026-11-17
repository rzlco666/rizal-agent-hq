use hq_connectors::probes::Probes;
use hq_connectors::runtime::{
    Clock, CollectCtx, ProcessPresenceAdapter, RuntimeAdapter, SystemHealthEmitter,
};
use hq_core::{EventDraft, EventKind};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

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

#[cfg(test)]
mod tests {
    use super::parse_repos_from_config;

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
}
