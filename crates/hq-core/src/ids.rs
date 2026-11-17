use crate::error::CoreError;
use ulid::Ulid;

pub type Ms = i64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Seq(pub i64);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AgentId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SessionId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AlertId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConnectorId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeviceId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActionId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventId(pub Ulid);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceId(pub String);

fn valid_id(s: &str, prefix: &str) -> Result<(), CoreError> {
    if s.len() < 3 || s.len() > 200 {
        return Err(CoreError::InvalidId(s.to_string()));
    }
    if !s.starts_with(prefix) {
        return Err(CoreError::InvalidId(s.to_string()));
    }
    if !s
        .chars()
        .all(|c| matches!(c, 'a'..='z' | '0'..='9' | ':' | '.' | '_' | '/' | '-'))
    {
        return Err(CoreError::InvalidId(s.to_string()));
    }
    if s.contains("..") {
        return Err(CoreError::InvalidId(s.to_string()));
    }
    Ok(())
}
impl TaskId {
    pub fn parse(s: &str) -> Result<Self, CoreError> {
        valid_id(s, "task:")?;
        Ok(Self(s.to_string()))
    }
}

impl AgentId {
    pub fn parse(s: &str) -> Result<Self, CoreError> {
        valid_id(s, "agent:")?;
        Ok(Self(s.to_string()))
    }
}
impl SessionId {
    pub fn parse(s: &str) -> Result<Self, CoreError> {
        valid_id(s, "agent:")?;
        if !s.contains("/session/") {
            return Err(CoreError::InvalidId(s.to_string()));
        }
        Ok(Self(s.to_string()))
    }
}
// constructors continue below

// We'll implement constructors manually:
impl ConnectorId {
    pub fn parse(s: &str) -> Result<Self, CoreError> {
        valid_id(s, "connector:")?;
        Ok(Self(s.to_string()))
    }
}
impl AlertId {
    pub fn parse(s: &str) -> Result<Self, CoreError> {
        valid_id(s, "alert:")?;
        Ok(Self(s.to_string()))
    }
}
impl DeviceId {
    pub fn parse(s: &str) -> Result<Self, CoreError> {
        valid_id(s, "device:")?;
        Ok(Self(s.to_string()))
    }
}
impl ActionId {
    pub fn parse(s: &str) -> Result<Self, CoreError> {
        valid_id(s, "action:")?;
        Ok(Self(s.to_string()))
    }
}

// serde
use serde::{Deserialize, Serialize};
impl Serialize for AgentId {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0)
    }
}
impl<'de> Deserialize<'de> for AgentId {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = String::deserialize(d)?;
        Self::parse(&v).map_err(serde::de::Error::custom)
    }
}
impl Serialize for SessionId {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0)
    }
}
impl<'de> Deserialize<'de> for SessionId {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = String::deserialize(d)?;
        Self::parse(&v).map_err(serde::de::Error::custom)
    }
}
impl Serialize for TaskId {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0)
    }
}
impl<'de> Deserialize<'de> for TaskId {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = String::deserialize(d)?;
        Self::parse(&v).map_err(serde::de::Error::custom)
    }
}
impl Serialize for AlertId {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0)
    }
}
impl<'de> Deserialize<'de> for AlertId {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = String::deserialize(d)?;
        Self::parse(&v).map_err(serde::de::Error::custom)
    }
}
impl Serialize for ConnectorId {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0)
    }
}
impl<'de> Deserialize<'de> for ConnectorId {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = String::deserialize(d)?;
        Self::parse(&v).map_err(serde::de::Error::custom)
    }
}
impl Serialize for DeviceId {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0)
    }
}
impl<'de> Deserialize<'de> for DeviceId {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = String::deserialize(d)?;
        Self::parse(&v).map_err(serde::de::Error::custom)
    }
}
impl Serialize for ActionId {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0)
    }
}
impl<'de> Deserialize<'de> for ActionId {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = String::deserialize(d)?;
        Self::parse(&v).map_err(serde::de::Error::custom)
    }
}
impl Serialize for SourceId {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0)
    }
}
impl<'de> Deserialize<'de> for SourceId {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(Self(String::deserialize(d)?))
    }
}
impl Serialize for Seq {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_i64(self.0)
    }
}
impl<'de> Deserialize<'de> for Seq {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(Self(i64::deserialize(d)?))
    }
}
