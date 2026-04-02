#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::match_same_arms)]
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
                from: format!("{from:?}"),
                to: format!("{to:?}"),
            }),
        }
    }

    #[must_use]
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
                write!(f, "Invalid transition from {from} to {to}")
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

    #[test]
    fn job_status_display_all_variants() {
        assert_eq!(format!("{}", JobStatus::Pending), "pending");
        assert_eq!(format!("{}", JobStatus::Processing), "processing");
        assert_eq!(format!("{}", JobStatus::Completed), "completed");
        assert_eq!(format!("{}", JobStatus::Failed), "failed");
    }

    #[test]
    fn job_status_transition_pending_to_completed_rejected() {
        let status = JobStatus::Pending;
        let result = status.transition(JobStatus::Completed);
        assert!(result.is_err());
    }

    #[test]
    fn job_status_transition_pending_to_failed_rejected() {
        let status = JobStatus::Pending;
        let result = status.transition(JobStatus::Failed);
        assert!(result.is_err());
    }

    #[test]
    fn job_status_transition_completed_to_processing_rejected() {
        let status = JobStatus::Completed;
        let result = status.transition(JobStatus::Processing);
        assert!(result.is_err());
    }

    #[test]
    fn job_status_debug() {
        let status = JobStatus::Pending;
        let debug = format!("{:?}", status);
        assert!(debug.contains("Pending"));
    }

    #[test]
    fn job_status_clone_and_eq() {
        let a = JobStatus::Pending;
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn job_status_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(JobStatus::Pending);
        set.insert(JobStatus::Processing);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn job_status_serde_roundtrip() {
        let statuses = [
            JobStatus::Pending,
            JobStatus::Processing,
            JobStatus::Completed,
            JobStatus::Failed,
        ];
        for status in &statuses {
            let json = serde_json::to_string(status).unwrap();
            let back: JobStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(*status, back);
        }
    }

    #[test]
    fn queue_error_invalid_transition_display() {
        let err = QueueError::InvalidTransition {
            from: "Pending".into(),
            to: "Completed".into(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("Pending") && msg.contains("Completed"));
    }

    #[test]
    fn queue_error_implements_std_error() {
        let err: Box<dyn std::error::Error> = Box::new(QueueError::InvalidTransition {
            from: "A".into(),
            to: "B".into(),
        });
        let _ = format!("{err:?}");
    }

    // --- Additional comprehensive tests ---

    #[test]
    fn job_status_transition_all_valid_pairs() {
        let valid_transitions = [
            (JobStatus::Pending, JobStatus::Processing),
            (JobStatus::Processing, JobStatus::Completed),
            (JobStatus::Processing, JobStatus::Failed),
        ];
        for (from, to) in &valid_transitions {
            assert!(
                from.transition(*to).is_ok(),
                "Transition from {:?} to {:?} should succeed",
                from,
                to
            );
        }
    }

    #[test]
    fn job_status_all_invalid_transitions() {
        let invalid = [
            (JobStatus::Pending, JobStatus::Pending),
            (JobStatus::Pending, JobStatus::Completed),
            (JobStatus::Pending, JobStatus::Failed),
            (JobStatus::Processing, JobStatus::Pending),
            (JobStatus::Processing, JobStatus::Processing),
            (JobStatus::Completed, JobStatus::Pending),
            (JobStatus::Completed, JobStatus::Processing),
            (JobStatus::Completed, JobStatus::Completed),
            (JobStatus::Completed, JobStatus::Failed),
            (JobStatus::Failed, JobStatus::Pending),
            (JobStatus::Failed, JobStatus::Processing),
            (JobStatus::Failed, JobStatus::Completed),
            (JobStatus::Failed, JobStatus::Failed),
        ];
        for (from, to) in &invalid {
            assert!(
                from.transition(*to).is_err(),
                "Transition from {:?} to {:?} should fail",
                from,
                to
            );
        }
    }

    #[test]
    fn job_status_transition_error_contains_state_names() {
        let result = JobStatus::Pending.transition(JobStatus::Failed);
        if let Err(QueueError::InvalidTransition { from, to }) = result {
            assert!(from.contains("Pending"));
            assert!(to.contains("Failed"));
        } else {
            panic!("Expected InvalidTransition error");
        }
    }

    #[test]
    fn job_status_is_terminal_consistency() {
        let terminal = [JobStatus::Completed, JobStatus::Failed];
        let non_terminal = [JobStatus::Pending, JobStatus::Processing];

        for status in &terminal {
            assert!(status.is_terminal(), "{:?} should be terminal", status);
        }
        for status in &non_terminal {
            assert!(!status.is_terminal(), "{:?} should not be terminal", status);
        }
    }

    #[test]
    fn job_status_serde_roundtrip_all_variants_via_value() {
        let statuses = [
            JobStatus::Pending,
            JobStatus::Processing,
            JobStatus::Completed,
            JobStatus::Failed,
        ];
        for status in &statuses {
            let value = serde_json::to_value(status).unwrap();
            let back: JobStatus = serde_json::from_value(value).unwrap();
            assert_eq!(*status, back);
        }
    }

    #[test]
    fn job_status_serde_deserialize_invalid_string_rejected() {
        let result: Result<JobStatus, _> = serde_json::from_str("\"InvalidStatus\"");
        assert!(result.is_err());
    }

    #[test]
    fn job_status_copy_semantics() {
        let a = JobStatus::Pending;
        let _b = a;
        assert_eq!(a, JobStatus::Pending);
    }

    #[test]
    fn job_status_equality_reflexive() {
        let status = JobStatus::Processing;
        assert_eq!(status, status);
    }

    #[test]
    fn job_status_equality_symmetric() {
        let a = JobStatus::Processing;
        let b = JobStatus::Processing;
        assert_eq!(a, b);
        assert_eq!(b, a);
    }

    #[test]
    fn job_status_hash_set_dedup() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for _ in 0..10 {
            set.insert(JobStatus::Pending);
        }
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn job_status_partial_eq_different_variants() {
        assert_ne!(JobStatus::Pending, JobStatus::Processing);
        assert_ne!(JobStatus::Processing, JobStatus::Completed);
        assert_ne!(JobStatus::Completed, JobStatus::Failed);
    }

    #[test]
    fn job_status_transition_processing_to_completed_returns_completed() {
        let result = JobStatus::Processing.transition(JobStatus::Completed);
        assert_eq!(result.unwrap(), JobStatus::Completed);
    }

    #[test]
    fn job_status_transition_processing_to_failed_returns_failed() {
        let result = JobStatus::Processing.transition(JobStatus::Failed);
        assert_eq!(result.unwrap(), JobStatus::Failed);
    }

    #[test]
    fn job_status_transition_pending_to_processing_returns_processing() {
        let result = JobStatus::Pending.transition(JobStatus::Processing);
        assert_eq!(result.unwrap(), JobStatus::Processing);
    }

    #[test]
    fn job_status_completed_to_failed_rejected() {
        assert!(JobStatus::Completed.transition(JobStatus::Failed).is_err());
    }

    #[test]
    fn job_status_failed_to_completed_rejected() {
        assert!(JobStatus::Failed.transition(JobStatus::Completed).is_err());
    }

    #[test]
    fn job_status_failed_to_processing_rejected() {
        assert!(JobStatus::Failed.transition(JobStatus::Processing).is_err());
    }

    // --- Proptests ---

    use proptest::prelude::*;
    use proptest::{prop_assert, prop_assert_eq};

    proptest! {
        #[test]
        fn proptest_job_status_serde_roundtrip(status in 0u8..4u8) {
            let original = match status {
                0 => JobStatus::Pending,
                1 => JobStatus::Processing,
                2 => JobStatus::Completed,
                _ => JobStatus::Failed,
            };
            let json = serde_json::to_string(&original).unwrap();
            let back: JobStatus = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(original, back);
        }

        #[test]
        fn proptest_job_status_terminal_no_transition(
            status in 0u8..2u8
        ) {
            let terminal = match status {
                0 => JobStatus::Completed,
                _ => JobStatus::Failed,
            };
            let targets = [JobStatus::Pending, JobStatus::Processing];
            for target in &targets {
                prop_assert!(terminal.transition(*target).is_err());
            }
        }

        #[test]
        fn proptest_job_status_display_always_nonempty(status in 0u8..4u8) {
            let s = match status {
                0 => JobStatus::Pending,
                1 => JobStatus::Processing,
                2 => JobStatus::Completed,
                _ => JobStatus::Failed,
            };
            let display = format!("{s}");
            prop_assert!(!display.is_empty());
        }
    }
}
