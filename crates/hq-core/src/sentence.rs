use crate::error::CoreError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sentence {
    pub key: String,
    pub params: BTreeMap<String, String>,
}

impl Sentence {
    pub fn new(
        key: impl Into<String>,
        params: BTreeMap<String, String>,
    ) -> Result<Self, CoreError> {
        let key = key.into();
        if key.is_empty() {
            return Err(CoreError::InvalidSentence("empty key".to_string()));
        }
        for k in params.keys() {
            if k.is_empty() {
                return Err(CoreError::InvalidSentence("empty param key".to_string()));
            }
        }
        Ok(Self { key, params })
    }
}
