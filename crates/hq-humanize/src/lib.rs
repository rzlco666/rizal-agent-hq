//! Fact-to-template rendering and task lifecycle projections.

#![forbid(unsafe_code)]
pub mod fact;
pub mod locale;
pub mod render;
pub mod tasks;
pub mod verbs;

pub use fact::*;
pub use hq_core::{Sentence, TaskPhase};
pub use locale::Locale;
pub use render::{render, templates};
pub use tasks::*;
pub use verbs::*;

#[derive(Debug, thiserror::Error)]
pub enum HumanizeError {
    #[error("unsupported locale: {0}")]
    UnsupportedLocale(String),
}
