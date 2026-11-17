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
        let guard = cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(v) = guard.get(&(kind, version)) {
            return Ok(v.clone());
        }
    }
    let schema_value: Value = match kind {
        EventKind::AgentObserved => serde_json::to_value(schema_for!(AgentObservedPayload))
            .map_err(|e| CoreError::Schema(e.to_string()))?,
        EventKind::AgentSeen => serde_json::to_value(schema_for!(AgentSeenPayload))
            .map_err(|e| CoreError::Schema(e.to_string()))?,
        EventKind::AgentStatusChanged => {
            serde_json::to_value(schema_for!(AgentStatusChangedPayload))
                .map_err(|e| CoreError::Schema(e.to_string()))?
        }
        EventKind::AgentDisappeared => serde_json::to_value(schema_for!(AgentDisappearedPayload))
            .map_err(|e| CoreError::Schema(e.to_string()))?,
        EventKind::SessionStarted => serde_json::to_value(schema_for!(SessionStartedPayload))
            .map_err(|e| CoreError::Schema(e.to_string()))?,
        EventKind::SessionUpdated => serde_json::to_value(schema_for!(SessionUpdatedPayload))
            .map_err(|e| CoreError::Schema(e.to_string()))?,
        EventKind::SessionEnded => serde_json::to_value(schema_for!(SessionEndedPayload))
            .map_err(|e| CoreError::Schema(e.to_string()))?,
        EventKind::LogRecorded => serde_json::to_value(schema_for!(LogRecordedPayload))
            .map_err(|e| CoreError::Schema(e.to_string()))?,
        EventKind::UsageRecorded => serde_json::to_value(schema_for!(UsageRecordedPayload))
            .map_err(|e| CoreError::Schema(e.to_string()))?,
        EventKind::QuotaObserved => serde_json::to_value(schema_for!(QuotaObservedPayload))
            .map_err(|e| CoreError::Schema(e.to_string()))?,
        EventKind::CostEstimated => serde_json::to_value(schema_for!(CostEstimatedPayload))
            .map_err(|e| CoreError::Schema(e.to_string()))?,
        EventKind::AlertRaised => serde_json::to_value(schema_for!(AlertRaisedPayload))
            .map_err(|e| CoreError::Schema(e.to_string()))?,
        EventKind::AlertUpdated => serde_json::to_value(schema_for!(AlertUpdatedPayload))
            .map_err(|e| CoreError::Schema(e.to_string()))?,
        EventKind::AlertAcknowledged => serde_json::to_value(schema_for!(AlertAcknowledgedPayload))
            .map_err(|e| CoreError::Schema(e.to_string()))?,
        EventKind::AlertSnoozed => serde_json::to_value(schema_for!(AlertSnoozedPayload))
            .map_err(|e| CoreError::Schema(e.to_string()))?,
        EventKind::AlertResolved => serde_json::to_value(schema_for!(AlertResolvedPayload))
            .map_err(|e| CoreError::Schema(e.to_string()))?,
        EventKind::ActionRequested => serde_json::to_value(schema_for!(ActionRequestedPayload))
            .map_err(|e| CoreError::Schema(e.to_string()))?,
        EventKind::ActionApproved => serde_json::to_value(schema_for!(ActionApprovedPayload))
            .map_err(|e| CoreError::Schema(e.to_string()))?,
        EventKind::ActionDenied => serde_json::to_value(schema_for!(ActionDeniedPayload))
            .map_err(|e| CoreError::Schema(e.to_string()))?,
        EventKind::ActionExecuted => serde_json::to_value(schema_for!(ActionExecutedPayload))
            .map_err(|e| CoreError::Schema(e.to_string()))?,
        EventKind::ActionFailed => serde_json::to_value(schema_for!(ActionFailedPayload))
            .map_err(|e| CoreError::Schema(e.to_string()))?,
        EventKind::ConnectorRegistered => {
            serde_json::to_value(schema_for!(ConnectorRegisteredPayload))
                .map_err(|e| CoreError::Schema(e.to_string()))?
        }
        EventKind::ConnectorHealthChanged => {
            serde_json::to_value(schema_for!(ConnectorHealthChangedPayload))
                .map_err(|e| CoreError::Schema(e.to_string()))?
        }
        EventKind::ConnectorPollCompleted => {
            serde_json::to_value(schema_for!(ConnectorPollCompletedPayload))
                .map_err(|e| CoreError::Schema(e.to_string()))?
        }
        EventKind::ConnectorAuthFailed => {
            serde_json::to_value(schema_for!(ConnectorAuthFailedPayload))
                .map_err(|e| CoreError::Schema(e.to_string()))?
        }
        EventKind::ConnectorEvent => serde_json::to_value(schema_for!(ConnectorEventPayload))
            .map_err(|e| CoreError::Schema(e.to_string()))?,
        EventKind::KnowledgePageSynced => {
            serde_json::to_value(schema_for!(KnowledgePageSyncedPayload))
                .map_err(|e| CoreError::Schema(e.to_string()))?
        }
        EventKind::KnowledgePageQuarantined => {
            serde_json::to_value(schema_for!(KnowledgePageQuarantinedPayload))
                .map_err(|e| CoreError::Schema(e.to_string()))?
        }
        EventKind::KnowledgePromoted => serde_json::to_value(schema_for!(KnowledgePromotedPayload))
            .map_err(|e| CoreError::Schema(e.to_string()))?,
        EventKind::SystemHealthObserved => {
            serde_json::to_value(schema_for!(SystemHealthObservedPayload))
                .map_err(|e| CoreError::Schema(e.to_string()))?
        }
        EventKind::ConfigChanged => serde_json::to_value(schema_for!(ConfigChangedPayload))
            .map_err(|e| CoreError::Schema(e.to_string()))?,
        EventKind::SecretRotated => serde_json::to_value(schema_for!(SecretRotatedPayload))
            .map_err(|e| CoreError::Schema(e.to_string()))?,
        EventKind::DeviceAdded => serde_json::to_value(schema_for!(DeviceAddedPayload))
            .map_err(|e| CoreError::Schema(e.to_string()))?,
        EventKind::DeviceRevoked => serde_json::to_value(schema_for!(DeviceRevokedPayload))
            .map_err(|e| CoreError::Schema(e.to_string()))?,
        EventKind::NotificationSent => serde_json::to_value(schema_for!(NotificationSentPayload))
            .map_err(|e| CoreError::Schema(e.to_string()))?,
        EventKind::TelemetryMetric => serde_json::to_value(schema_for!(TelemetryMetricPayload))
            .map_err(|e| CoreError::Schema(e.to_string()))?,
        EventKind::TelemetryTrace => serde_json::to_value(schema_for!(TelemetryTracePayload))
            .map_err(|e| CoreError::Schema(e.to_string()))?,
        EventKind::TelemetryLog => serde_json::to_value(schema_for!(TelemetryLogPayload))
            .map_err(|e| CoreError::Schema(e.to_string()))?,
    };
    let mut guard = cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    guard.insert((kind, version), schema_value.clone());
    Ok(schema_value)
}
