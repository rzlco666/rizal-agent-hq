use crate::runtime::omp::slug;
use hq_core::{
    AgentId, AgentKind, EventDraft, EventKind, EvidenceLevel, Sentence, SourceId, Status, Subject,
    redact_text,
};
use serde_json::json;
use std::cmp::Reverse;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_TERMINAL_BYTES: u64 = 1024 * 1024;
const MAX_TERMINALS: usize = 20;
const MAX_WORKSPACES: usize = 5;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CursorTerminal {
    pub pid: Option<u32>,
    pub cwd: Option<String>,
    pub command: Option<String>,
    pub title: String,
    pub status: String,
    pub started_at: Option<String>,
    pub running_for_ms: Option<u64>,
}

/// Reads the simple `---` header used by Cursor terminal transcripts. This is
/// deliberately not a full YAML parser; only scalar `key: value` pairs are used.
pub fn parse_cursor_terminal(input: &str) -> Option<CursorTerminal> {
    let mut lines = input.lines();
    if lines.next()?.trim() != "---" {
        return None;
    }
    let mut terminal = CursorTerminal::default();
    for line in lines {
        let line = line.trim();
        if line == "---" {
            break;
        }
        let (key, value) = line.split_once(':')?;
        let value = value.trim().trim_matches('"').trim_matches('\'');
        match key.trim() {
            "pid" => terminal.pid = value.parse().ok(),
            "cwd" => terminal.cwd = Some(value.to_owned()),
            "command" => terminal.command = Some(redact_text(value)),
            "title" => terminal.title = value.to_owned(),
            "status" => terminal.status = value.to_owned(),
            "started_at" => terminal.started_at = Some(value.to_owned()),
            "running_for_ms" => terminal.running_for_ms = value.parse().ok(),
            _ => {}
        }
    }
    (!terminal.title.is_empty() && !terminal.status.is_empty()).then_some(terminal)
}

struct TerminalView {
    modified: SystemTime,
    terminal: CursorTerminal,
}

pub struct CursorAdapter {
    root: PathBuf,
}

impl CursorAdapter {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    pub fn observe_dirs(&self, now_ms: i64) -> Vec<EventDraft> {
        let mut terminals = self.terminals();
        terminals.sort_by_key(|terminal| Reverse(terminal.0));
        terminals.truncate(MAX_TERMINALS);

        if terminals.is_empty() {
            return Vec::new();
        }

        let mut by_workspace: BTreeMap<String, TerminalView> = BTreeMap::new();
        for (modified, workspace, terminal) in &terminals {
            by_workspace
                .entry(workspace.clone())
                .or_insert_with(|| TerminalView {
                    modified: *modified,
                    terminal: terminal.clone(),
                });
        }
        let mut workspaces: Vec<(String, TerminalView)> = by_workspace.into_iter().collect();
        workspaces.sort_by_key(|workspace| Reverse(workspace.1.modified));
        workspaces.truncate(MAX_WORKSPACES);

        let workspace_count = self.workspace_count();
        let mut events = Vec::new();
        for (workspace, view) in &workspaces {
            events.push(workspace_event(
                now_ms,
                workspace,
                &view.terminal,
                workspace_count,
            ));
        }
        let aggregate_status = if terminals
            .iter()
            .any(|(_, _, terminal)| cursor_status(&terminal.status) == Status::Error)
        {
            Status::Error
        } else if terminals
            .iter()
            .any(|(_, _, terminal)| cursor_status(&terminal.status) == Status::Bekerja)
        {
            Status::Bekerja
        } else {
            Status::Santai
        };
        events.push(aggregate_event(
            now_ms,
            aggregate_status,
            workspaces.len(),
            terminals.len(),
        ));
        events
    }

    fn workspace_count(&self) -> usize {
        fs::read_dir(self.root.join("projects"))
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .filter(|entry| entry.path().is_dir())
                    .count()
            })
            .unwrap_or_default()
    }

    fn terminals(&self) -> Vec<(SystemTime, String, CursorTerminal)> {
        let mut found = Vec::new();
        let Ok(projects) = fs::read_dir(self.root.join("projects")) else {
            return found;
        };
        for project in projects.filter_map(Result::ok) {
            if !project.path().is_dir() {
                continue;
            }
            let workspace = slug(&project.file_name().to_string_lossy());
            let Ok(terminals) = fs::read_dir(project.path().join("terminals")) else {
                continue;
            };
            for entry in terminals.filter_map(Result::ok) {
                let path = entry.path();
                if path.extension().is_none_or(|ext| ext != "txt") {
                    continue;
                }
                let Ok(metadata) = fs::metadata(&path) else {
                    continue;
                };
                if metadata.len() > MAX_TERMINAL_BYTES {
                    continue;
                }
                let Ok(contents) = fs::read_to_string(&path) else {
                    continue;
                };
                if let Some(terminal) = parse_cursor_terminal(&contents) {
                    found.push((
                        metadata.modified().unwrap_or(UNIX_EPOCH),
                        workspace.clone(),
                        terminal,
                    ));
                }
            }
        }
        found
    }
}

#[async_trait::async_trait]
impl crate::runtime::RuntimeAdapter for CursorAdapter {
    async fn observe(&self, ctx: &crate::runtime::CollectCtx) -> Result<Vec<EventDraft>, String> {
        Ok(self.observe_dirs(ctx.now_ms))
    }
}

fn cursor_status(status: &str) -> Status {
    match status.trim().to_ascii_lowercase().as_str() {
        "running" => Status::Bekerja,
        "failed" | "error" | "errored" => Status::Error,
        _ => Status::Santai,
    }
}

fn activity(title: &str) -> Sentence {
    let mut params = BTreeMap::new();
    params.insert(
        "title".to_owned(),
        redact_text(title).chars().take(200).collect(),
    );
    Sentence {
        key: "agent.cursor_activity".to_owned(),
        params,
    }
}

fn workspace_event(
    now_ms: i64,
    workspace: &str,
    terminal: &CursorTerminal,
    workspace_count: usize,
) -> EventDraft {
    let status = cursor_status(&terminal.status);
    EventDraft {
        event_ts_ms: now_ms,
        kind: EventKind::AgentObserved,
        subject: Subject::Agent(AgentId(format!("agent:cursor/workspace/{workspace}"))),
        source: SourceId("local".to_owned()),
        correlation_id: None,
        causation_id: None,
        idempotency_key: Some(format!(
            "cursor:workspace:{workspace}:{:?}:{}",
            status,
            now_ms / 60_000
        )),
        payload: json!({
            "observed_at_ms": now_ms,
            "runtime_key": format!("cursor:workspace:{workspace}"),
            "status": status,
            "kind": AgentKind::Utama,
            "activity": activity(&terminal.title),
            "processes": 0_u32,
            "cpu_percent": 0.0,
            "evidence": EvidenceLevel::L2,
            "workspace": workspace,
            "workspaces_seen": workspace_count,
            "terminal_status": terminal.status,
            "source": "cursor terminal header",
        }),
    }
}

fn aggregate_event(now_ms: i64, status: Status, workspaces: usize, terminals: usize) -> EventDraft {
    EventDraft {
        event_ts_ms: now_ms,
        kind: EventKind::AgentObserved,
        subject: Subject::Agent(AgentId("agent:cursor".to_owned())),
        source: SourceId("local".to_owned()),
        correlation_id: None,
        causation_id: None,
        idempotency_key: Some(format!("cursor:aggregate:{:?}:{}", status, now_ms / 60_000)),
        payload: json!({
            "observed_at_ms": now_ms,
            "runtime_key": "cursor:workspaces",
            "status": status,
            "kind": AgentKind::Utama,
            "activity": activity("Cursor workspaces"),
            "processes": 0_u32,
            "cpu_percent": 0.0,
            "evidence": EvidenceLevel::L2,
            "workspaces_seen": workspaces,
            "terminals_seen": terminals,
            "source": "cursor terminal header",
        }),
    }
}
