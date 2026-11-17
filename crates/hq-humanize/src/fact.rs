use crate::Verb;
use hq_core::{Sentence, redact_text};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Raw activity reported by a runtime adapter. All free text is redacted
/// before it becomes a sentence parameter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fact {
    pub agent: String,
    pub verb: Verb,
    pub object: Option<String>,
    pub detail: Option<String>,
    pub outcome: Option<String>,
}

impl Fact {
    pub fn new(agent: impl Into<String>, verb: Verb) -> Self {
        Self {
            agent: agent.into(),
            verb,
            object: None,
            detail: None,
            outcome: None,
        }
    }
}

/// Map a fact to a stable template key plus redacted parameters.
pub fn humanize(fact: &Fact) -> Sentence {
    let mut params = BTreeMap::new();
    params.insert("agent".to_string(), redact_text(&fact.agent));
    for (name, value) in [
        ("object", &fact.object),
        ("detail", &fact.detail),
        ("outcome", &fact.outcome),
    ] {
        if let Some(value) = value {
            let redacted = redact_text(value);
            if !redacted.is_empty() {
                params.insert(name.to_string(), redacted);
            }
        }
    }
    Sentence {
        key: fact.verb.key().to_string(),
        params,
    }
}

/// Severity carried by a log event; stored as a lowercase token.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    #[default]
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn parse(input: &str) -> Self {
        match input.trim().to_ascii_lowercase().as_str() {
            "debug" => Self::Debug,
            "warn" | "warning" => Self::Warn,
            "error" => Self::Error,
            _ => Self::Info,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}
