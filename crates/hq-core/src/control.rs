use crate::ids::AgentId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Actor {
    Owner,
    Automation(String),
    Agent(AgentId),
    System,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Capability {
    AgentPrompt,
    AgentPause,
    AgentResume,
    AgentStop,
    ToolApprove,
    ToolDeny,
    ConnectorAction(String),
    ShellRun(String),
    ConfigWrite,
    DeviceManage,
    McpControl,
}

impl Capability {
    pub fn parse(s: &str) -> Result<Self, crate::error::CoreError> {
        match s {
            "agent.prompt" => Ok(Self::AgentPrompt),
            "agent.pause" => Ok(Self::AgentPause),
            "agent.resume" => Ok(Self::AgentResume),
            "agent.stop" => Ok(Self::AgentStop),
            "tool.approve" => Ok(Self::ToolApprove),
            "tool.deny" => Ok(Self::ToolDeny),
            "config.write" => Ok(Self::ConfigWrite),
            "device.manage" => Ok(Self::DeviceManage),
            "mcp.control" => Ok(Self::McpControl),
            _ => {
                if let Some(name) = s.strip_prefix("connector.action:") {
                    if name.is_empty() {
                        return Err(crate::error::CoreError::InvalidCapability(s.to_string()));
                    }
                    return Ok(Self::ConnectorAction(name.to_string()));
                }
                if let Some(name) = s.strip_prefix("shell.run:") {
                    if name.is_empty() {
                        return Err(crate::error::CoreError::InvalidCapability(s.to_string()));
                    }
                    return Ok(Self::ShellRun(name.to_string()));
                }
                Err(crate::error::CoreError::InvalidCapability(s.to_string()))
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyDecision {
    Allow,
    RequireApproval,
    Deny,
}
