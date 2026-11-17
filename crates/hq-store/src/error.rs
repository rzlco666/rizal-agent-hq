use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("pool: {0}")]
    Pool(String),
    #[error("validation: {0}")]
    Validation(String),
    #[error("migration: {0}")]
    Migration(String),
    #[error("not found")]
    NotFound,
    #[error("channel closed")]
    ChannelClosed,
    #[error("core: {0}")]
    Core(String),
}
impl From<hq_core::CoreError> for StoreError {
    fn from(e: hq_core::CoreError) -> Self {
        StoreError::Core(e.to_string())
    }
}
