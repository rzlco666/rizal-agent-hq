use hq_store::reads::AlertView as StoreAlertView;
use hq_store::{
    AgentCard, AgentDetail, ChangedFile as StoreChangedFile, CostSummary, Event,
    EventPage as StoreEventPage, LogEntry, SessionView as StoreSessionView,
    TaskView as StoreTaskView, UsageTotals,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentCardDto {
    pub id: String,
    pub name: String,
    pub status: String,
}

impl From<AgentCard> for AgentCardDto {
    fn from(card: AgentCard) -> Self {
        Self {
            id: card.id,
            name: card.name,
            status: format!("{:?}", card.status).to_lowercase(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Counts {
    pub working: i64,
    pub waiting: i64,
    pub offline: i64,
    pub alerts_open: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Overview {
    pub agents: Vec<AgentCardDto>,
    pub counts: Counts,
    pub seq: i64,
    pub usage_requests: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentPage {
    pub items: Vec<AgentCardDto>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentDetailDto {
    pub card: AgentCardDto,
    pub as_of: Option<i64>,
    pub tasks: Vec<TaskView>,
    pub sessions: Vec<SessionView>,
}

impl From<AgentDetail> for AgentDetailDto {
    fn from(detail: AgentDetail) -> Self {
        Self {
            card: detail.card.into(),
            as_of: None,
            tasks: Vec::new(),
            sessions: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TaskView {
    pub id: String,
    pub agent_id: String,
    pub session_id: Option<String>,
    pub title: String,
    pub phase: String,
    pub started_ms: i64,
    pub updated_ms: i64,
    pub ended_ms: Option<i64>,
}

impl From<StoreTaskView> for TaskView {
    fn from(task: StoreTaskView) -> Self {
        Self {
            id: task.id,
            agent_id: task.agent_id,
            session_id: task.session_id,
            title: task.title,
            phase: task.phase,
            started_ms: task.started_ms,
            updated_ms: task.updated_ms,
            ended_ms: task.ended_ms,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TaskPage {
    pub items: Vec<TaskView>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LogView {
    pub id: String,
    pub ts_ms: i64,
    pub level: String,
    pub text: String,
}

impl From<LogEntry> for LogView {
    fn from(entry: LogEntry) -> Self {
        Self {
            id: entry.id,
            ts_ms: entry.ts_ms,
            level: format!("{:?}", entry.level).to_lowercase(),
            text: entry.text,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LogPage {
    pub items: Vec<LogView>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SessionView {
    pub id: String,
    pub agent_id: String,
    pub runtime: String,
    pub started_ms: i64,
    pub ended_ms: Option<i64>,
    pub outcome: Option<String>,
    pub requests: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cash_usd: f64,
}

impl From<StoreSessionView> for SessionView {
    fn from(session: StoreSessionView) -> Self {
        Self {
            id: session.id,
            agent_id: session.agent_id,
            runtime: session.runtime,
            started_ms: session.started_ms,
            ended_ms: session.ended_ms,
            outcome: session.outcome,
            requests: session.requests,
            input_tokens: session.input_tokens,
            output_tokens: session.output_tokens,
            cash_usd: session.cash_usd,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SessionPage {
    pub items: Vec<SessionView>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChangedFile {
    pub path: String,
    pub agent_id: String,
    pub changed_ms: i64,
}

impl From<StoreChangedFile> for ChangedFile {
    fn from(file: StoreChangedFile) -> Self {
        Self {
            path: file.path,
            agent_id: file.agent_id,
            changed_ms: file.changed_ms,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AlertView {
    pub id: String,
    pub fingerprint: String,
    pub rule_id: String,
    pub level: String,
    pub state: String,
    pub subject: String,
    pub title: String,
    pub opened_ms: i64,
    pub updated_ms: i64,
    pub ack_ms: Option<i64>,
    pub snooze_until_ms: Option<i64>,
    pub resolved_ms: Option<i64>,
    pub occurrences: i64,
}

impl From<StoreAlertView> for AlertView {
    fn from(alert: StoreAlertView) -> Self {
        Self {
            id: alert.id,
            fingerprint: alert.fingerprint,
            rule_id: alert.rule_id,
            level: alert.level,
            state: alert.state,
            subject: alert.subject,
            title: alert.title,
            opened_ms: alert.opened_ms,
            updated_ms: alert.updated_ms,
            ack_ms: alert.ack_ms,
            snooze_until_ms: alert.snooze_until_ms,
            resolved_ms: alert.resolved_ms,
            occurrences: alert.occurrences,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AlertActionResult {
    pub id: String,
    pub state: String,
    pub seq: i64,
    pub snooze_until_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EventView {
    pub seq: i64,
    pub kind: String,
    pub subject: String,
    pub source: String,
    pub event_ts_ms: i64,
    pub payload: serde_json::Value,
}

impl From<Event> for EventView {
    fn from(event: Event) -> Self {
        Self {
            seq: event.seq.0,
            kind: event.draft.kind.as_str().to_string(),
            subject: event.draft.subject.key(),
            source: event.draft.source.0,
            event_ts_ms: event.draft.event_ts_ms,
            payload: event.draft.payload,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EventPage {
    pub items: Vec<EventView>,
    pub next_cursor: Option<String>,
}

impl From<StoreEventPage> for EventPage {
    fn from(page: StoreEventPage) -> Self {
        Self {
            items: page.events.into_iter().map(EventView::from).collect(),
            next_cursor: page.next_cursor.map(|seq| seq.to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UsageTotalsDto {
    pub requests: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_tokens: i64,
    pub cash_usd: f64,
    pub shadow_usd: f64,
}

impl From<UsageTotals> for UsageTotalsDto {
    fn from(total: UsageTotals) -> Self {
        Self {
            requests: total.requests,
            input_tokens: total.input_tokens,
            output_tokens: total.output_tokens,
            cache_tokens: total.cache_tokens,
            cash_usd: total.cash_usd,
            shadow_usd: total.shadow_usd,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UsageReportDto {
    pub total: UsageTotalsDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CostReport {
    pub cash_usd: f64,
    pub shadow_usd: f64,
}

impl From<CostSummary> for CostReport {
    fn from(cost: CostSummary) -> Self {
        Self {
            cash_usd: cost.cash_usd,
            shadow_usd: cost.shadow_usd,
        }
    }
}
