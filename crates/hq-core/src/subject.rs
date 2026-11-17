use crate::ids::{ActionId, AgentId, AlertId, ConnectorId, DeviceId, SessionId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum UsageScope {
    Global,
    Agent(AgentId),
    Session(SessionId),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Subject {
    Agent(AgentId),
    Session(SessionId),
    Connector(ConnectorId),
    Alert(AlertId),
    Device(DeviceId),
    Action(ActionId),
    Knowledge(String),
    Usage(UsageScope),
    System,
}

impl Subject {
    pub fn key(&self) -> String {
        match self {
            Subject::Agent(id) => id.0.clone(),
            Subject::Session(id) => id.0.clone(),
            Subject::Connector(id) => id.0.clone(),
            Subject::Alert(id) => id.0.clone(),
            Subject::Device(id) => id.0.clone(),
            Subject::Action(id) => id.0.clone(),
            Subject::Knowledge(s) => format!("knowledge:{}", s),
            Subject::Usage(u) => match u {
                UsageScope::Global => "usage:global".to_string(),
                UsageScope::Agent(a) => {
                    format!("usage:agent:{}", a.0.strip_prefix("agent:").unwrap_or(&a.0))
                }
                UsageScope::Session(s) => format!("usage:session:{}", s.0),
            },
            Subject::System => "system".to_string(),
        }
    }
}

// serde impls
impl Serialize for UsageScope {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        // not specified; use debug-ish? Keep simple via key not needed. Use manual tag.
        match self {
            UsageScope::Global => s.serialize_str("global"),
            UsageScope::Agent(a) => s.serialize_str(&format!("agent:{}", a.0)),
            UsageScope::Session(sess) => s.serialize_str(&format!("session:{}", sess.0)),
        }
    }
}
impl<'de> Deserialize<'de> for UsageScope {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = String::deserialize(d)?;
        if v == "global" {
            return Ok(UsageScope::Global);
        }
        if let Some(rest) = v.strip_prefix("agent:") {
            return Ok(UsageScope::Agent(
                AgentId::parse(rest).map_err(serde::de::Error::custom)?,
            ));
        }
        if let Some(rest) = v.strip_prefix("session:") {
            return Ok(UsageScope::Session(
                SessionId::parse(rest).map_err(serde::de::Error::custom)?,
            ));
        }
        Err(serde::de::Error::custom("bad usage scope"))
    }
}
impl Serialize for Subject {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.key())
    }
}
impl<'de> Deserialize<'de> for Subject {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        // not fully specified; accept key and reconstruct minimally
        let v = String::deserialize(d)?;
        // best-effort
        if v == "system" {
            return Ok(Subject::System);
        }
        if v == "usage:global" {
            return Ok(Subject::Usage(UsageScope::Global));
        }
        if let Some(rest) = v.strip_prefix("usage:agent:") {
            return Ok(Subject::Usage(UsageScope::Agent(
                AgentId::parse(&format!("agent:{}", rest)).map_err(serde::de::Error::custom)?,
            )));
        }
        if let Some(rest) = v.strip_prefix("usage:session:") {
            return Ok(Subject::Usage(UsageScope::Session(
                SessionId::parse(rest).map_err(serde::de::Error::custom)?,
            )));
        }
        if let Some(rest) = v.strip_prefix("knowledge:") {
            return Ok(Subject::Knowledge(rest.to_string()));
        }
        // try ids
        if let Ok(a) = AgentId::parse(&v) {
            return Ok(Subject::Agent(a));
        }
        if let Ok(sess) = SessionId::parse(&v) {
            return Ok(Subject::Session(sess));
        }
        if let Ok(c) = ConnectorId::parse(&v) {
            return Ok(Subject::Connector(c));
        }
        if let Ok(a) = AlertId::parse(&v) {
            return Ok(Subject::Alert(a));
        }
        if let Ok(d) = DeviceId::parse(&v) {
            return Ok(Subject::Device(d));
        }
        if let Ok(a) = ActionId::parse(&v) {
            return Ok(Subject::Action(a));
        }
        Err(serde::de::Error::custom("bad subject"))
    }
}
