use crate::error::CoreError;
use crate::event::EventKind;
use serde_json::Value;

pub type UpcastFn = fn(Value) -> Result<Value, CoreError>;

pub fn registry(kind: EventKind, from_version: u16) -> Option<UpcastFn> {
    // Versi saat ini adalah 1. Penambahan versi baru wajib menambah tes migrasi payload lama.
    if from_version == 1 {
        Some(Ok)
    } else {
        let _ = kind;
        None
    }
}

pub fn upcast(kind: EventKind, from_version: u16, payload: Value) -> Result<Value, CoreError> {
    let f = registry(kind, from_version).ok_or(CoreError::UnsupportedVersion(from_version))?;
    f(payload)
}
