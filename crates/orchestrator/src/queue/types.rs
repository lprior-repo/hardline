//! Queue domain types
//!
//! Core data types for the job processing queue:
//! - JobPriority: Priority levels P0-P4
//! - JobState: Lifecycle states
//! - Job: Main job entity
//! - JobPayload: Job data variants
//! - JobResult: Execution result
//! - JobOutcome: Success or failure

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobPriority {
    P0,
    P1,
    P2,
    P3,
    P4,
}

impl JobPriority {
    #[must_use]
    pub fn value(&self) -> u8 {
        match self {
            Self::P0 => 0,
            Self::P1 => 1,
            Self::P2 => 2,
            Self::P3 => 3,
            Self::P4 => 4,
        }
    }

    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::P0,
            1 => Self::P1,
            2 => Self::P2,
            3 => Self::P3,
            _ => Self::P4,
        }
    }
}

impl std::fmt::Display for JobPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "P{}", self.value())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Pending,
    Running {
        started_at: DateTime<Utc>,
    },
    Completed {
        finished_at: DateTime<Utc>,
    },
    Failed {
        error: String,
        failed_at: DateTime<Utc>,
    },
}

impl JobState {
    #[must_use]
    pub fn is_pending(&self) -> bool {
        matches!(self, Self::Pending)
    }

    #[must_use]
    pub fn is_running(&self) -> bool {
        matches!(self, Self::Running { .. })
    }

    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed { .. } | Self::Failed { .. })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum JobPayload {
    Pipeline { spec_path: String },
    Task { command: String },
    Custom { data: serde_json::Value },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub priority: JobPriority,
    pub payload: JobPayload,
    pub state: JobState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResult {
    pub job_id: String,
    pub outcome: JobOutcome,
    pub execution_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum JobOutcome {
    Success,
    Failure { error: String },
}
