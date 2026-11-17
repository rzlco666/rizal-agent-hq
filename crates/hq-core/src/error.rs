use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("schema validation failed: {0}")]
    Schema(String),
    #[error("invalid id: {0}")]
    InvalidId(String),
    #[error("invalid sentence: {0}")]
    InvalidSentence(String),
    #[error("invalid capability: {0}")]
    InvalidCapability(String),
    #[error("unsupported version: {0}")]
    UnsupportedVersion(u16),
    #[error("validation failed: {0}")]
    Validation(String),
}
