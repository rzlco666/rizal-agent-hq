use crate::error::CoreError;
use crate::ids::{EventId, Ms, Seq, SourceId};
use crate::subject::Subject;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    AgentObserved,
    AgentSeen,
    AgentStatusChanged,
    AgentDisappeared,
    SessionStarted,
    SessionUpdated,
    SessionEnded,
    LogRecorded,
    UsageRecorded,
    QuotaObserved,
    CostEstimated,
    AlertRaised,
    AlertUpdated,
    AlertAcknowledged,
    AlertSnoozed,
    AlertResolved,
    ActionRequested,
    ActionApproved,
    ActionDenied,
    ActionExecuted,
    ActionFailed,
    ConnectorRegistered,
    ConnectorHealthChanged,
    ConnectorPollCompleted,
    ConnectorAuthFailed,
    ConnectorEvent,
    KnowledgePageSynced,
    KnowledgePageQuarantined,
    KnowledgePromoted,
    SystemHealthObserved,
    ConfigChanged,
    SecretRotated,
    DeviceAdded,
    DeviceRevoked,
    NotificationSent,
    TelemetryMetric,
    TelemetryTrace,
    TelemetryLog,
}

impl EventKind {
    pub const ALL: &'static [EventKind] = &[
        EventKind::AgentObserved,
        EventKind::AgentSeen,
        EventKind::AgentStatusChanged,
        EventKind::AgentDisappeared,
        EventKind::SessionStarted,
        EventKind::SessionUpdated,
        EventKind::SessionEnded,
        EventKind::LogRecorded,
        EventKind::UsageRecorded,
        EventKind::QuotaObserved,
        EventKind::CostEstimated,
        EventKind::AlertRaised,
        EventKind::AlertUpdated,
        EventKind::AlertAcknowledged,
        EventKind::AlertSnoozed,
        EventKind::AlertResolved,
        EventKind::ActionRequested,
        EventKind::ActionApproved,
        EventKind::ActionDenied,
        EventKind::ActionExecuted,
        EventKind::ActionFailed,
        EventKind::ConnectorRegistered,
        EventKind::ConnectorHealthChanged,
        EventKind::ConnectorPollCompleted,
        EventKind::ConnectorAuthFailed,
        EventKind::ConnectorEvent,
        EventKind::KnowledgePageSynced,
        EventKind::KnowledgePageQuarantined,
        EventKind::KnowledgePromoted,
        EventKind::SystemHealthObserved,
        EventKind::ConfigChanged,
        EventKind::SecretRotated,
        EventKind::DeviceAdded,
        EventKind::DeviceRevoked,
        EventKind::NotificationSent,
        EventKind::TelemetryMetric,
        EventKind::TelemetryTrace,
        EventKind::TelemetryLog,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            EventKind::AgentObserved => "agent_observed",
            EventKind::AgentSeen => "agent_seen",
            EventKind::AgentStatusChanged => "agent_status_changed",
            EventKind::AgentDisappeared => "agent_disappeared",
            EventKind::SessionStarted => "session_started",
            EventKind::SessionUpdated => "session_updated",
            EventKind::SessionEnded => "session_ended",
            EventKind::LogRecorded => "log_recorded",
            EventKind::UsageRecorded => "usage_recorded",
            EventKind::QuotaObserved => "quota_observed",
            EventKind::CostEstimated => "cost_estimated",
            EventKind::AlertRaised => "alert_raised",
            EventKind::AlertUpdated => "alert_updated",
            EventKind::AlertAcknowledged => "alert_acknowledged",
            EventKind::AlertSnoozed => "alert_snoozed",
            EventKind::AlertResolved => "alert_resolved",
            EventKind::ActionRequested => "action_requested",
            EventKind::ActionApproved => "action_approved",
            EventKind::ActionDenied => "action_denied",
            EventKind::ActionExecuted => "action_executed",
            EventKind::ActionFailed => "action_failed",
            EventKind::ConnectorRegistered => "connector_registered",
            EventKind::ConnectorHealthChanged => "connector_health_changed",
            EventKind::ConnectorPollCompleted => "connector_poll_completed",
            EventKind::ConnectorAuthFailed => "connector_auth_failed",
            EventKind::ConnectorEvent => "connector_event",
            EventKind::KnowledgePageSynced => "knowledge_page_synced",
            EventKind::KnowledgePageQuarantined => "knowledge_page_quarantined",
            EventKind::KnowledgePromoted => "knowledge_promoted",
            EventKind::SystemHealthObserved => "system_health_observed",
            EventKind::ConfigChanged => "config_changed",
            EventKind::SecretRotated => "secret_rotated",
            EventKind::DeviceAdded => "device_added",
            EventKind::DeviceRevoked => "device_revoked",
            EventKind::NotificationSent => "notification_sent",
            EventKind::TelemetryMetric => "telemetry_metric",
            EventKind::TelemetryTrace => "telemetry_trace",
            EventKind::TelemetryLog => "telemetry_log",
        }
    }
}

impl AsRef<EventKind> for EventKind {
    fn as_ref(&self) -> &EventKind {
        self
    }
}

#[derive(Debug, Clone)]
pub struct EventDraft {
    pub event_ts_ms: Ms,
    pub kind: EventKind,
    pub subject: Subject,
    pub source: SourceId,
    pub correlation_id: Option<EventId>,
    pub causation_id: Option<EventId>,
    pub idempotency_key: Option<String>,
    pub payload: Value,
}

impl EventDraft {
    pub fn test_with_payload(kind: EventKind, payload: Value) -> Self {
        Self {
            event_ts_ms: 1,
            kind,
            subject: Subject::System,
            source: SourceId("test".to_string()),
            correlation_id: None,
            causation_id: None,
            idempotency_key: None,
            payload,
        }
    }

    pub fn validate(&self) -> Result<(), CoreError> {
        if self.event_ts_ms <= 0 {
            return Err(CoreError::Validation("event_ts_ms must be > 0".to_string()));
        }
        if self.subject.key().is_empty() {
            return Err(CoreError::Validation(
                "subject must not be empty".to_string(),
            ));
        }
        if !self.payload.is_object() {
            return Err(CoreError::Schema("payload must be an object".to_string()));
        }
        let schema = crate::schema::schema_for(self.kind, 1)?;
        let validator =
            jsonschema::validator_for(&schema).map_err(|e| CoreError::Schema(e.to_string()))?;
        if let Err(e) = validator.validate(&self.payload) {
            return Err(CoreError::Schema(format!("{} at {}", e, e.instance_path())));
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Event {
    pub seq: Seq,
    pub id: EventId,
    pub ingest_ts_ms: Ms,
    pub draft: EventDraft,
}
