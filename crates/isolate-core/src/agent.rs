//! Agent coordination - Multi-agent support from Stak
//!
//! This module provides agent coordination for 100+ agent support.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::queue::SessionName;

/// Heartbeat timeout in seconds
const HEARTBEAT_TIMEOUT_SECS: i64 = 60;

/// Unique agent identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(String);

impl AgentId {
    pub fn new(id: impl Into<String>) -> Result<Self, Error> {
        let id = id.into();
        if id.is_empty() {
            Err(Error::InvalidId("AgentId cannot be empty".into()))
        } else {
            Ok(Self(id))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Agent activity state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AgentActivity {
    #[default]
    Idle,
    Working {
        session: SessionName,
        command: String,
    },
}

impl AgentActivity {
    pub fn is_working(&self) -> bool {
        matches!(self, Self::Working { .. })
    }

    pub fn session(&self) -> Option<&SessionName> {
        match self {
            Self::Idle => None,
            Self::Working { session, .. } => Some(session),
        }
    }

    pub fn command(&self) -> Option<&str> {
        match self {
            Self::Idle => None,
            Self::Working { command, .. } => Some(command),
        }
    }
}

/// Agent status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    Active,
    Stale,
}

impl std::fmt::Display for AgentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Stale => write!(f, "stale"),
        }
    }
}

/// An agent in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: AgentId,
    pub registered_at: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub activity: AgentActivity,
    pub actions_count: u64,
}

impl Agent {
    pub fn new(id: AgentId) -> Self {
        let now = Utc::now();
        Self {
            id,
            registered_at: now,
            last_seen: now,
            activity: AgentActivity::default(),
            actions_count: 0,
        }
    }

    pub fn is_active(&self) -> bool {
        let now = Utc::now();
        (now - self.last_seen).num_seconds() < HEARTBEAT_TIMEOUT_SECS
    }

    pub fn status(&self) -> AgentStatus {
        if self.is_active() {
            AgentStatus::Active
        } else {
            AgentStatus::Stale
        }
    }

    pub fn update_heartbeat(&mut self) {
        self.last_seen = Utc::now();
    }

    pub fn start_work(&mut self, session: SessionName, command: String) {
        self.activity = AgentActivity::Working { session, command };
        self.actions_count += 1;
    }

    pub fn stop_work(&mut self) {
        self.activity = AgentActivity::Idle;
    }
}
