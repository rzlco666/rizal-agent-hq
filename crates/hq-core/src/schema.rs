use crate::error::CoreError;
use crate::event::EventKind;
use crate::payload::*;
use schemars::schema_for;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

static CACHE: OnceLock<Mutex<HashMap<(EventKind, u16), Value>>> = OnceLock::new();

pub fn schema_for<K: AsRef<EventKind>>(kind: K, version: u16) -> Result<Value, CoreError> {
    let kind = *kind.as_ref();
    if version != 1 {
        return Err(CoreError::UnsupportedVersion(version));
    }
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    {
        let guard = cache.lock().unwrap();
        if let Some(v) = guard.get(&(kind, version)) {
            return Ok(v.clone());
        }
    }
    let schema_value: Value = match kind {
        EventKind::AgentObserved => {
            serde_json::to_value(schema_for!(AgentObservedPayload)).unwrap()
        }
        EventKind::AgentSeen => serde_json::to_value(schema_for!(AgentSeenPayload)).unwrap(),
        EventKind::AgentStatusChanged => {
            serde_json::to_value(schema_for!(AgentStatusChangedPayload)).unwrap()
        }
        EventKind::AgentDisappeared => {
            serde_json::to_value(schema_for!(AgentDisappearedPayload)).unwrap()
        }
        EventKind::SessionStarted => {
            serde_json::to_value(schema_for!(SessionStartedPayload)).unwrap()
        }
        EventKind::SessionUpdated => {
            serde_json::to_value(schema_for!(SessionUpdatedPayload)).unwrap()
        }
        EventKind::SessionEnded => serde_json::to_value(schema_for!(SessionEndedPayload)).unwrap(),
        EventKind::LogRecorded => serde_json::to_value(schema_for!(LogRecordedPayload)).unwrap(),
        EventKind::UsageRecorded => {
            serde_json::to_value(schema_for!(UsageRecordedPayload)).unwrap()
        }
        EventKind::QuotaObserved => {
            serde_json::to_value(schema_for!(QuotaObservedPayload)).unwrap()
        }
        EventKind::CostEstimated => {
            serde_json::to_value(schema_for!(CostEstimatedPayload)).unwrap()
        }
        EventKind::AlertRaised => serde_json::to_value(schema_for!(AlertRaisedPayload)).unwrap(),
        EventKind::AlertUpdated => serde_json::to_value(schema_for!(AlertUpdatedPayload)).unwrap(),
        EventKind::AlertAcknowledged => {
            serde_json::to_value(schema_for!(AlertAcknowledgedPayload)).unwrap()
        }
        EventKind::AlertSnoozed => serde_json::to_value(schema_for!(AlertSnoozedPayload)).unwrap(),
        EventKind::AlertResolved => {
            serde_json::to_value(schema_for!(AlertResolvedPayload)).unwrap()
        }
        EventKind::ActionRequested => {
            serde_json::to_value(schema_for!(ActionRequestedPayload)).unwrap()
        }
        EventKind::ActionApproved => {
            serde_json::to_value(schema_for!(ActionApprovedPayload)).unwrap()
        }
        EventKind::ActionDenied => serde_json::to_value(schema_for!(ActionDeniedPayload)).unwrap(),
        EventKind::ActionExecuted => {
            serde_json::to_value(schema_for!(ActionExecutedPayload)).unwrap()
        }
        EventKind::ActionFailed => serde_json::to_value(schema_for!(ActionFailedPayload)).unwrap(),
        EventKind::ConnectorRegistered => {
            serde_json::to_value(schema_for!(ConnectorRegisteredPayload)).unwrap()
        }
        EventKind::ConnectorHealthChanged => {
            serde_json::to_value(schema_for!(ConnectorHealthChangedPayload)).unwrap()
        }
        EventKind::ConnectorPollCompleted => {
            serde_json::to_value(schema_for!(ConnectorPollCompletedPayload)).unwrap()
        }
        EventKind::ConnectorAuthFailed => {
            serde_json::to_value(schema_for!(ConnectorAuthFailedPayload)).unwrap()
        }
        EventKind::ConnectorEvent => {
            serde_json::to_value(schema_for!(ConnectorEventPayload)).unwrap()
        }
        EventKind::KnowledgePageSynced => {
            serde_json::to_value(schema_for!(KnowledgePageSyncedPayload)).unwrap()
        }
        EventKind::KnowledgePageQuarantined => {
            serde_json::to_value(schema_for!(KnowledgePageQuarantinedPayload)).unwrap()
        }
        EventKind::KnowledgePromoted => {
            serde_json::to_value(schema_for!(KnowledgePromotedPayload)).unwrap()
        }
        EventKind::SystemHealthObserved => {
            serde_json::to_value(schema_for!(SystemHealthObservedPayload)).unwrap()
        }
        EventKind::ConfigChanged => {
            serde_json::to_value(schema_for!(ConfigChangedPayload)).unwrap()
        }
        EventKind::SecretRotated => {
            serde_json::to_value(schema_for!(SecretRotatedPayload)).unwrap()
        }
        EventKind::DeviceAdded => serde_json::to_value(schema_for!(DeviceAddedPayload)).unwrap(),
        EventKind::DeviceRevoked => {
            serde_json::to_value(schema_for!(DeviceRevokedPayload)).unwrap()
        }
        EventKind::NotificationSent => {
            serde_json::to_value(schema_for!(NotificationSentPayload)).unwrap()
        }
        EventKind::TelemetryMetric => {
            serde_json::to_value(schema_for!(TelemetryMetricPayload)).unwrap()
        }
        EventKind::TelemetryTrace => {
            serde_json::to_value(schema_for!(TelemetryTracePayload)).unwrap()
        }
        EventKind::TelemetryLog => serde_json::to_value(schema_for!(TelemetryLogPayload)).unwrap(),
    };
    let mut guard = cache.lock().unwrap();
    guard.insert((kind, version), schema_value.clone());
    Ok(schema_value)
}
