mod en;
mod id;
mod zh;

use crate::HumanizeError;
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum Locale {
    #[default]
    Id,
    En,
    Zh,
}

impl Locale {
    pub fn parse(input: &str) -> Result<Self, HumanizeError> {
        match input.trim().to_ascii_lowercase().as_str() {
            "id" => Ok(Self::Id),
            "en" => Ok(Self::En),
            "zh" => Ok(Self::Zh),
            _ => Err(HumanizeError::UnsupportedLocale(input.to_string())),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Id => "id",
            Self::En => "en",
            Self::Zh => "zh",
        }
    }

    pub(crate) fn template(self, key: &str) -> Option<&'static str> {
        match self {
            Self::Id => id::template(key),
            Self::En => en::template(key),
            Self::Zh => zh::template(key),
        }
    }
}
