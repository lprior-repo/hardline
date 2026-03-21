#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum JobStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

impl JobStatus {
    pub fn transition(&self, to: JobStatus) -> Result<JobStatus, QueueError> {
        match (self, to) {
            (JobStatus::Pending, JobStatus::Processing) => Ok(to),
            (JobStatus::Processing, JobStatus::Completed) => Ok(to),
            (JobStatus::Processing, JobStatus::Failed) => Ok(to),
            (from, to) => Err(QueueError::InvalidTransition {
                from: format!("{:?}", from),
                to: format!("{:?}", to),
            }),
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, JobStatus::Completed | JobStatus::Failed)
    }
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Processing => write!(f, "processing"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum QueueError {
    InvalidTransition { from: String, to: String },
}

impl std::fmt::Display for QueueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTransition { from, to } => {
                write!(f, "Invalid transition from {} to {}", from, to)
            }
        }
    }
}

impl std::error::Error for QueueError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_status_transition_pending_to_processing() {
        let status = JobStatus::Pending;
        let result = status.transition(JobStatus::Processing);
        assert!(result.is_ok());
    }

    #[test]
    fn job_status_transition_processing_to_completed() {
        let status = JobStatus::Processing;
        let result = status.transition(JobStatus::Completed);
        assert!(result.is_ok());
    }

    #[test]
    fn job_status_transition_processing_to_failed() {
        let status = JobStatus::Processing;
        let result = status.transition(JobStatus::Failed);
        assert!(result.is_ok());
    }

    #[test]
    fn job_status_transition_same_state_rejected() {
        let status = JobStatus::Pending;
        let result = status.transition(JobStatus::Pending);
        assert!(result.is_err());
    }

    #[test]
    fn job_status_transition_from_completed_rejected() {
        let status = JobStatus::Completed;
        let result = status.transition(JobStatus::Processing);
        assert!(result.is_err());
    }

    #[test]
    fn job_status_transition_from_failed_rejected() {
        let status = JobStatus::Failed;
        let result = status.transition(JobStatus::Processing);
        assert!(result.is_err());
    }

    #[test]
    fn job_status_is_terminal() {
        assert!(JobStatus::Completed.is_terminal());
        assert!(JobStatus::Failed.is_terminal());
        assert!(!JobStatus::Pending.is_terminal());
        assert!(!JobStatus::Processing.is_terminal());
    }
}
