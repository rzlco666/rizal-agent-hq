use crate::error::StoreError;
use hq_core::*;
use rusqlite::Transaction;

pub trait Projector: Send + Sync {
    fn name(&self) -> &'static str;
    fn reset(&self, tx: &Transaction<'_>) -> Result<(), StoreError>;
    fn apply(&self, tx: &Transaction<'_>, ev: &Event) -> Result<(), StoreError>;
}

pub mod agents;
pub mod alerts;
pub mod logs;
pub mod system_health;
pub mod tasks;
pub mod usage;

pub fn apply_agent(tx: &Transaction<'_>, ev: &Event) -> Result<(), StoreError> {
    agents::apply(tx, ev)
}
pub fn apply_usage(tx: &Transaction<'_>, ev: &Event) -> Result<(), StoreError> {
    usage::apply(tx, ev)
}
pub fn apply_alert(tx: &Transaction<'_>, ev: &Event) -> Result<(), StoreError> {
    alerts::apply(tx, ev)
}
pub fn apply_system_health(tx: &Transaction<'_>, ev: &Event) -> Result<(), StoreError> {
    system_health::apply(tx, ev)
}
