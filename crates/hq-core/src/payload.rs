use crate::{AgentKind, EvidenceLevel, Sentence, Status};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct AgentObservedPayload {
    pub observed_at_ms: i64,
    pub runtime_key: String,
    pub status: Status,
    pub kind: AgentKind,
    pub activity: Sentence,
    pub processes: u32,
    pub cpu_percent: f64,
    pub evidence: EvidenceLevel,
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct AgentSeenPayload {
    pub seen_at_ms: i64,
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct AgentStatusChangedPayload {
    pub status: String,
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct AgentDisappearedPayload {
    pub disappeared_at_ms: i64,
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct SessionStartedPayload {
    pub started_at_ms: i64,
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct SessionUpdatedPayload {
    pub updated_at_ms: i64,
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct SessionEndedPayload {
    pub ended_at_ms: i64,
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct LogRecordedPayload {
    pub message: String,
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct UsageRecordedPayload {
    pub tokens: i64,
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct QuotaObservedPayload {
    pub remaining: i64,
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct CostEstimatedPayload {
    pub cost_usd: f64,
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct AlertRaisedPayload {
    pub message: String,
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct AlertUpdatedPayload {
    pub message: String,
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct AlertAcknowledgedPayload {
    pub at_ms: i64,
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct AlertSnoozedPayload {
    pub until_ms: i64,
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct AlertResolvedPayload {
    pub at_ms: i64,
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct ActionRequestedPayload {
    pub action_id: String,
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct ActionApprovedPayload {
    pub action_id: String,
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct ActionDeniedPayload {
    pub action_id: String,
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct ActionExecutedPayload {
    pub action_id: String,
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct ActionFailedPayload {
    pub action_id: String,
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct ConnectorRegisteredPayload {
    pub connector_id: String,
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct ConnectorHealthChangedPayload {
    pub status: String,
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct ConnectorPollCompletedPayload {
    pub items: i64,
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct ConnectorAuthFailedPayload {
    pub reason: String,
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct ConnectorEventPayload {
    pub data: String,
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct KnowledgePageSyncedPayload {
    pub page_id: String,
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct KnowledgePageQuarantinedPayload {
    pub page_id: String,
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct KnowledgePromotedPayload {
    pub page_id: String,
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct SystemHealthObservedPayload {
    pub cpu_percent: f64,
    pub mem_percent: f64,
    pub disk_percent: f64,
    pub uptime_secs: u64,
    pub docker: Vec<serde_json::Value>,
    pub git: Vec<serde_json::Value>,
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct ConfigChangedPayload {
    pub key: String,
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct SecretRotatedPayload {
    pub name: String,
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct DeviceAddedPayload {
    pub device_id: String,
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct DeviceRevokedPayload {
    pub device_id: String,
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct NotificationSentPayload {
    pub channel: String,
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct TelemetryMetricPayload {
    pub value: f64,
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct TelemetryTracePayload {
    pub trace_id: String,
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct TelemetryLogPayload {
    pub message: String,
}
