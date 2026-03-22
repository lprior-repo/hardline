//! State types for durable workflow execution

use serde::{Deserialize, Serialize};

// =============================================================================
// Operation State Machine
// =============================================================================

/// State of a durable operation (tracks multi-step AI workflows)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationState {
    /// Operation created, not yet running
    Started,
    /// Currently executing
    InProgress,
    /// Successfully finished
    Completed,
    /// Permanently failed (no more retries)
    Failed,
}

impl OperationState {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Started => "started",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }

    #[must_use]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "started" => Some(Self::Started),
            "in_progress" => Some(Self::InProgress),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }

    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed)
    }
}

/// hardline-specific operation status with more states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationStatus {
    /// Operation created, waiting to start
    Pending,
    /// Currently executing steps
    Running,
    /// All steps completed successfully
    Completed,
    /// Failed with error (may have partial compensation)
    Failed,
    /// Waiting on external input (promise/awakeable)
    Suspended,
    /// Compensation in progress (rolling back)
    Compensating,
}

impl OperationStatus {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Suspended => "suspended",
            Self::Compensating => "compensating",
        }
    }

    #[must_use]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "running" => Some(Self::Running),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            "suspended" => Some(Self::Suspended),
            "compensating" => Some(Self::Compensating),
            _ => None,
        }
    }

    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed)
    }
}

// =============================================================================
// Step Journal
// =============================================================================

/// Status of a single step within an operation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    /// Step not yet started
    Pending,
    /// Step currently executing
    Running,
    /// Step completed successfully
    Completed,
    /// Step failed
    Failed,
    /// Skipped due to earlier failure (compensation)
    Skipped,
}

impl StepStatus {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
        }
    }

    #[must_use]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "running" => Some(Self::Running),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            "skipped" => Some(Self::Skipped),
            _ => None,
        }
    }

    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Skipped)
    }
}

// =============================================================================
// Journal Structure
// =============================================================================

/// Journal entry states for two-phase compensation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JournalState {
    /// Pending external work
    PendingExternal,
    /// Compensation in progress (rolling back)
    Compensating,
    /// Operation completed successfully
    Done,
    /// Compensation failed (needs manual intervention)
    FailedCompensation,
}

impl JournalState {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PendingExternal => "pending_external",
            Self::Compensating => "compensating",
            Self::Done => "done",
            Self::FailedCompensation => "failed_compensation",
        }
    }

    #[must_use]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending_external" => Some(Self::PendingExternal),
            "compensating" => Some(Self::Compensating),
            "done" => Some(Self::Done),
            "failed_compensation" => Some(Self::FailedCompensation),
            _ => None,
        }
    }
}

// =============================================================================
// Two-Phase Compensation
// =============================================================================

/// Compensation state machine for saga pattern
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompensationState {
    /// Operation succeeded, no compensation needed
    NoCompensationNeeded,
    /// Currently rolling back
    CompensationInProgress,
    /// Rollback succeeded
    CompensationCompleted,
    /// Rollback failed (needs manual intervention)
    CompensationFailed,
}

impl CompensationState {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NoCompensationNeeded => "no_compensation_needed",
            Self::CompensationInProgress => "compensation_in_progress",
            Self::CompensationCompleted => "compensation_completed",
            Self::CompensationFailed => "compensation_failed",
        }
    }

    #[must_use]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "no_compensation_needed" => Some(Self::NoCompensationNeeded),
            "compensation_in_progress" => Some(Self::CompensationInProgress),
            "compensation_completed" => Some(Self::CompensationCompleted),
            "compensation_failed" => Some(Self::CompensationFailed),
            _ => None,
        }
    }

    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::NoCompensationNeeded | Self::CompensationCompleted | Self::CompensationFailed
        )
    }
}

// =============================================================================
// Pipeline State Machine (Orchestrator)
// =============================================================================

/// Pipeline state machine states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineState {
    /// Initial state - pipeline created but not started
    Pending,
    /// Running linter on spec
    SpecReview,
    /// Deploying twin/universe
    UniverseSetup,
    /// Agent working (with iteration count)
    AgentDevelopment,
    /// Running scenarios for validation
    Validation,
    /// All scenarios passed - artifact ready for merge
    Accepted,
    /// Human intervention needed
    Escalated,
    /// Validation failed permanently
    Failed,
}

impl PipelineState {
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            PipelineState::Accepted | PipelineState::Escalated | PipelineState::Failed
        )
    }

    #[must_use]
    pub fn allows_iteration(&self) -> bool {
        matches!(self, PipelineState::AgentDevelopment)
    }

    #[must_use]
    pub fn description(&self) -> &'static str {
        match self {
            PipelineState::Pending => "Pending - awaiting start",
            PipelineState::SpecReview => "Spec Review - running linter",
            PipelineState::UniverseSetup => "Universe Setup - deploying twin",
            PipelineState::AgentDevelopment => "Agent Development - working on task",
            PipelineState::Validation => "Validation - running scenarios",
            PipelineState::Accepted => "Accepted - all scenarios passed",
            PipelineState::Escalated => "Escalated - human intervention needed",
            PipelineState::Failed => "Failed - validation failed",
        }
    }
}

impl std::fmt::Display for PipelineState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}
