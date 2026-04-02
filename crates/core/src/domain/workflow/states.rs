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
    pub const fn as_str(&self) -> &'static str {
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
    pub const fn is_terminal(&self) -> bool {
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
    pub const fn as_str(&self) -> &'static str {
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
    pub const fn is_terminal(&self) -> bool {
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
    pub const fn as_str(&self) -> &'static str {
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
    pub const fn is_terminal(&self) -> bool {
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
    pub const fn as_str(&self) -> &'static str {
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
    pub const fn as_str(&self) -> &'static str {
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
    pub const fn is_terminal(&self) -> bool {
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
    pub const fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Accepted | Self::Escalated | Self::Failed
        )
    }

    #[must_use]
    pub const fn allows_iteration(&self) -> bool {
        matches!(self, Self::AgentDevelopment)
    }

    #[must_use]
    pub const fn description(&self) -> &'static str {
        match self {
            Self::Pending => "Pending - awaiting start",
            Self::SpecReview => "Spec Review - running linter",
            Self::UniverseSetup => "Universe Setup - deploying twin",
            Self::AgentDevelopment => "Agent Development - working on task",
            Self::Validation => "Validation - running scenarios",
            Self::Accepted => "Accepted - all scenarios passed",
            Self::Escalated => "Escalated - human intervention needed",
            Self::Failed => "Failed - validation failed",
        }
    }
}

impl std::fmt::Display for PipelineState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // PipelineState::is_terminal
    // -------------------------------------------------------------------------

    #[test]
    fn terminal_states_are_accepted_escalated_and_failed() {
        assert!(PipelineState::Accepted.is_terminal());
        assert!(PipelineState::Escalated.is_terminal());
        assert!(PipelineState::Failed.is_terminal());
    }

    #[test]
    fn non_terminal_states_report_false() {
        assert!(!PipelineState::Pending.is_terminal());
        assert!(!PipelineState::SpecReview.is_terminal());
        assert!(!PipelineState::UniverseSetup.is_terminal());
        assert!(!PipelineState::AgentDevelopment.is_terminal());
        assert!(!PipelineState::Validation.is_terminal());
    }

    // -------------------------------------------------------------------------
    // PipelineState::allows_iteration
    // -------------------------------------------------------------------------

    #[test]
    fn only_agent_development_allows_iteration() {
        assert!(PipelineState::AgentDevelopment.allows_iteration());
    }

    #[test]
    fn no_other_state_allows_iteration() {
        let non_iteration_states = [
            PipelineState::Pending,
            PipelineState::SpecReview,
            PipelineState::UniverseSetup,
            PipelineState::Validation,
            PipelineState::Accepted,
            PipelineState::Escalated,
            PipelineState::Failed,
        ];
        for state in non_iteration_states {
            assert!(
                !state.allows_iteration(),
                "{state:?} should not allow iteration"
            );
        }
    }

    // -------------------------------------------------------------------------
    // PipelineState::description
    // -------------------------------------------------------------------------

    #[test]
    fn description_returns_non_empty_string_for_all_variants() {
        let states = [
            PipelineState::Pending,
            PipelineState::SpecReview,
            PipelineState::UniverseSetup,
            PipelineState::AgentDevelopment,
            PipelineState::Validation,
            PipelineState::Accepted,
            PipelineState::Escalated,
            PipelineState::Failed,
        ];
        for state in states {
            assert!(
                !state.description().is_empty(),
                "{state:?} description should not be empty"
            );
            assert!(
                state.description().contains(" - "),
                "{state:?} description should contain ' - ' separator"
            );
        }
    }

    // -------------------------------------------------------------------------
    // PipelineState::Display
    // -------------------------------------------------------------------------

    #[test]
    fn display_delegates_to_description() {
        assert_eq!(
            PipelineState::Pending.to_string(),
            PipelineState::Pending.description()
        );
        assert_eq!(
            PipelineState::Failed.to_string(),
            PipelineState::Failed.description()
        );
    }

    // -------------------------------------------------------------------------
    // OperationState
    // -------------------------------------------------------------------------

    #[test]
    fn operation_state_roundtrip_str() {
        let states = [
            OperationState::Started,
            OperationState::InProgress,
            OperationState::Completed,
            OperationState::Failed,
        ];
        for state in states {
            assert_eq!(OperationState::from_str(state.as_str()), Some(state));
        }
    }

    #[test]
    fn operation_state_unknown_str_returns_none() {
        assert!(OperationState::from_str("bogus").is_none());
        assert!(OperationState::from_str("").is_none());
    }

    #[test]
    fn operation_state_terminal_check() {
        assert!(OperationState::Completed.is_terminal());
        assert!(OperationState::Failed.is_terminal());
        assert!(!OperationState::Started.is_terminal());
        assert!(!OperationState::InProgress.is_terminal());
    }

    // -------------------------------------------------------------------------
    // OperationStatus
    // -------------------------------------------------------------------------

    #[test]
    fn operation_status_roundtrip_str() {
        let statuses = [
            OperationStatus::Pending,
            OperationStatus::Running,
            OperationStatus::Completed,
            OperationStatus::Failed,
            OperationStatus::Suspended,
            OperationStatus::Compensating,
        ];
        for status in statuses {
            assert_eq!(
                OperationStatus::from_str(status.as_str()),
                Some(status)
            );
        }
    }

    #[test]
    fn operation_status_terminal_check() {
        assert!(OperationStatus::Completed.is_terminal());
        assert!(OperationStatus::Failed.is_terminal());
        assert!(!OperationStatus::Pending.is_terminal());
        assert!(!OperationStatus::Running.is_terminal());
        assert!(!OperationStatus::Suspended.is_terminal());
        assert!(!OperationStatus::Compensating.is_terminal());
    }

    // -------------------------------------------------------------------------
    // StepStatus
    // -------------------------------------------------------------------------

    #[test]
    fn step_status_roundtrip_str() {
        let statuses = [
            StepStatus::Pending,
            StepStatus::Running,
            StepStatus::Completed,
            StepStatus::Failed,
            StepStatus::Skipped,
        ];
        for status in statuses {
            assert_eq!(StepStatus::from_str(status.as_str()), Some(status));
        }
    }

    #[test]
    fn step_status_terminal_check() {
        assert!(StepStatus::Completed.is_terminal());
        assert!(StepStatus::Failed.is_terminal());
        assert!(StepStatus::Skipped.is_terminal());
        assert!(!StepStatus::Pending.is_terminal());
        assert!(!StepStatus::Running.is_terminal());
    }

    // -------------------------------------------------------------------------
    // JournalState
    // -------------------------------------------------------------------------

    #[test]
    fn journal_state_roundtrip_str() {
        let states = [
            JournalState::PendingExternal,
            JournalState::Compensating,
            JournalState::Done,
            JournalState::FailedCompensation,
        ];
        for state in states {
            assert_eq!(JournalState::from_str(state.as_str()), Some(state));
        }
    }

    // -------------------------------------------------------------------------
    // CompensationState
    // -------------------------------------------------------------------------

    #[test]
    fn compensation_state_roundtrip_str() {
        let states = [
            CompensationState::NoCompensationNeeded,
            CompensationState::CompensationInProgress,
            CompensationState::CompensationCompleted,
            CompensationState::CompensationFailed,
        ];
        for state in states {
            assert_eq!(
                CompensationState::from_str(state.as_str()),
                Some(state)
            );
        }
    }

    #[test]
    fn compensation_state_terminal_check() {
        assert!(CompensationState::NoCompensationNeeded.is_terminal());
        assert!(CompensationState::CompensationCompleted.is_terminal());
        assert!(CompensationState::CompensationFailed.is_terminal());
        assert!(!CompensationState::CompensationInProgress.is_terminal());
    }
}
