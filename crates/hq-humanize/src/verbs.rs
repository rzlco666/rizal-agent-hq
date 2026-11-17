use serde::{Deserialize, Serialize};

/// Closed set of actions that can be turned into a human sentence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verb {
    StartedTask,
    RunningTests,
    Building,
    EditingFile,
    Reading,
    Searching,
    Committing,
    WaitingUser,
    Completed,
    Failed,
    Idle,
    Thinking,
    Meeting,
    Serving,
}

impl Verb {
    pub const ALL: &'static [Self] = &[
        Self::StartedTask,
        Self::RunningTests,
        Self::Building,
        Self::EditingFile,
        Self::Reading,
        Self::Searching,
        Self::Committing,
        Self::WaitingUser,
        Self::Completed,
        Self::Failed,
        Self::Idle,
        Self::Thinking,
        Self::Meeting,
        Self::Serving,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::StartedTask => "started_task",
            Self::RunningTests => "running_tests",
            Self::Building => "building",
            Self::EditingFile => "editing_file",
            Self::Reading => "reading",
            Self::Searching => "searching",
            Self::Committing => "committing",
            Self::WaitingUser => "waiting_user",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Idle => "idle",
            Self::Thinking => "thinking",
            Self::Meeting => "meeting",
            Self::Serving => "serving",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::StartedTask => "agent.started_task",
            Self::RunningTests => "agent.running_tests",
            Self::Building => "agent.building",
            Self::EditingFile => "agent.editing_file",
            Self::Reading => "agent.reading",
            Self::Searching => "agent.searching",
            Self::Committing => "agent.committing",
            Self::WaitingUser => "agent.waiting_user",
            Self::Completed => "agent.completed",
            Self::Failed => "agent.failed",
            Self::Idle => "agent.idle",
            Self::Thinking => "agent.thinking",
            Self::Meeting => "agent.meeting",
            Self::Serving => "agent.serving",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|verb| verb.key() == key)
    }
}
