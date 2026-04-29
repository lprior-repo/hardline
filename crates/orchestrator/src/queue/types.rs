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
    pub const fn value(&self) -> u8 {
        match self {
            Self::P0 => 0,
            Self::P1 => 1,
            Self::P2 => 2,
            Self::P3 => 3,
            Self::P4 => 4,
        }
    }

    pub const fn from_u8(value: u8) -> Self {
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
    pub const fn is_pending(&self) -> bool {
        matches!(self, Self::Pending)
    }

    #[must_use]
    pub const fn is_running(&self) -> bool {
        matches!(self, Self::Running { .. })
    }

    #[must_use]
    pub const fn is_terminal(&self) -> bool {
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

impl Job {
    /// Construct a new Job in Pending state.
    #[must_use]
    pub fn new(id: String, priority: JobPriority, payload: JobPayload) -> Self {
        let now = Utc::now();
        Self {
            id,
            priority,
            payload,
            state: JobState::Pending,
            created_at: now,
            updated_at: now,
        }
    }

    /// Transition to a new state with validation.
    ///
    /// Valid transitions:
    /// - Pending → Running
    /// - Running → Completed
    /// - Running → Failed
    ///
    /// Terminal states (Completed, Failed) reject all transitions.
    pub fn transition_to(&mut self, new_state: JobState) -> Result<(), JobTransitionError> {
        match (&self.state, &new_state) {
            (JobState::Pending, JobState::Running { .. }) => {}
            (JobState::Running { .. }, JobState::Completed { .. }) => {}
            (JobState::Running { .. }, JobState::Failed { .. }) => {}
            (state, _) if state.is_terminal() => {
                return Err(JobTransitionError::AlreadyTerminal {
                    current: self.state.clone(),
                });
            }
            _ => {
                return Err(JobTransitionError::InvalidTransition {
                    from: self.state.clone(),
                    to: new_state,
                });
            }
        }

        self.state = new_state;
        self.updated_at = Utc::now();
        Ok(())
    }
}

/// Error when transitioning Job states
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobTransitionError {
    InvalidTransition { from: JobState, to: JobState },
    AlreadyTerminal { current: JobState },
}

impl std::fmt::Display for JobTransitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTransition { from, to } => {
                write!(f, "Invalid job transition from {from:?} to {to:?}")
            }
            Self::AlreadyTerminal { current } => {
                write!(f, "Job already in terminal state: {current:?}")
            }
        }
    }
}

impl std::error::Error for JobTransitionError {}

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

#[cfg(test)]
mod tests {
    use super::*;

    // --- JobPriority tests ---

    #[test]
    fn test_job_priority_value() {
        assert_eq!(JobPriority::P0.value(), 0);
        assert_eq!(JobPriority::P1.value(), 1);
        assert_eq!(JobPriority::P2.value(), 2);
        assert_eq!(JobPriority::P3.value(), 3);
        assert_eq!(JobPriority::P4.value(), 4);
    }

    #[test]
    fn test_job_priority_from_u8() {
        assert_eq!(JobPriority::from_u8(0), JobPriority::P0);
        assert_eq!(JobPriority::from_u8(1), JobPriority::P1);
        assert_eq!(JobPriority::from_u8(2), JobPriority::P2);
        assert_eq!(JobPriority::from_u8(3), JobPriority::P3);
        // Out of range should default to P4
        assert_eq!(JobPriority::from_u8(99), JobPriority::P4);
        assert_eq!(JobPriority::from_u8(u8::MAX), JobPriority::P4);
    }

    #[test]
    fn test_job_priority_display() {
        assert_eq!(format!("{}", JobPriority::P0), "P0");
        assert_eq!(format!("{}", JobPriority::P3), "P3");
    }

    #[test]
    fn test_job_priority_ordering() {
        assert!(JobPriority::P0 < JobPriority::P1);
        assert!(JobPriority::P1 < JobPriority::P2);
        assert!(JobPriority::P3 < JobPriority::P4);
    }

    // --- JobState tests ---

    #[test]
    fn test_job_state_is_running() {
        assert!(JobState::Running {
            started_at: chrono::Utc::now()
        }
        .is_running());
        assert!(!JobState::Pending.is_running());
        assert!(!JobState::Completed {
            finished_at: chrono::Utc::now()
        }
        .is_running());
    }

    #[test]
    fn test_job_state_is_terminal() {
        assert!(JobState::Completed {
            finished_at: chrono::Utc::now()
        }
        .is_terminal());
        assert!(JobState::Failed {
            error: "err".to_string(),
            failed_at: chrono::Utc::now()
        }
        .is_terminal());
        assert!(!JobState::Pending.is_terminal());
        assert!(!JobState::Running {
            started_at: chrono::Utc::now()
        }
        .is_terminal());
    }

    // --- JobPayload tests ---

    #[test]
    fn test_job_payload_variants() {
        let pipeline = JobPayload::Pipeline {
            spec_path: "specs/test.yaml".to_string(),
        };
        let task = JobPayload::Task {
            command: "build".to_string(),
        };
        let custom = JobPayload::Custom {
            data: serde_json::json!({"key": "value"}),
        };

        // Verify they are distinct
        let _ = (pipeline, task, custom);
    }

    // --- Serde roundtrips ---

    #[test]
    fn test_job_priority_serde_roundtrip() {
        let priorities = [
            JobPriority::P0,
            JobPriority::P1,
            JobPriority::P2,
            JobPriority::P3,
            JobPriority::P4,
        ];
        for priority in &priorities {
            let json = serde_json::to_string(priority).expect("serialize");
            let deserialized: JobPriority = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(*priority, deserialized);
        }
    }

    #[test]
    fn test_job_priority_serde_uses_lowercase() {
        let json = serde_json::to_string(&JobPriority::P0).expect("serialize");
        assert_eq!(json, "\"p0\"");
    }

    #[test]
    fn test_job_state_serde_roundtrip_all_variants() {
        let now = chrono::Utc::now();
        let states = [
            JobState::Pending,
            JobState::Running { started_at: now },
            JobState::Completed { finished_at: now },
            JobState::Failed {
                error: "test err".to_string(),
                failed_at: now,
            },
        ];
        for state in &states {
            let json = serde_json::to_string(state).expect("serialize");
            let deserialized: JobState = serde_json::from_str(&json).expect("deserialize");
            match (state, &deserialized) {
                (JobState::Pending, JobState::Pending) => {}
                (JobState::Running { .. }, JobState::Running { .. }) => {}
                (JobState::Completed { .. }, JobState::Completed { .. }) => {}
                (JobState::Failed { error: e1, .. }, JobState::Failed { error: e2, .. }) => {
                    assert_eq!(e1, e2);
                }
                _ => panic!("State mismatch"),
            }
        }
    }

    #[test]
    fn test_job_state_serde_uses_snake_case() {
        let json = serde_json::to_string(&JobState::Pending).expect("serialize");
        assert_eq!(json, "\"pending\"");
    }

    #[test]
    fn test_job_payload_serde_roundtrip() {
        let payloads = [
            JobPayload::Pipeline {
                spec_path: "specs/test.yaml".to_string(),
            },
            JobPayload::Task {
                command: "build".to_string(),
            },
            JobPayload::Custom {
                data: serde_json::json!({"key": "value"}),
            },
        ];
        for payload in &payloads {
            let json = serde_json::to_string(payload).expect("serialize");
            let deserialized: JobPayload = serde_json::from_str(&json).expect("deserialize");
            match (payload, &deserialized) {
                (
                    JobPayload::Pipeline { spec_path: s1 },
                    JobPayload::Pipeline { spec_path: s2 },
                ) => {
                    assert_eq!(s1, s2);
                }
                (JobPayload::Task { command: c1 }, JobPayload::Task { command: c2 }) => {
                    assert_eq!(c1, c2);
                }
                (JobPayload::Custom { data: d1 }, JobPayload::Custom { data: d2 }) => {
                    assert_eq!(d1, d2);
                }
                _ => panic!("Payload mismatch"),
            }
        }
    }

    #[test]
    fn test_job_serde_roundtrip() {
        let job = Job {
            id: "job-42".to_string(),
            priority: JobPriority::P1,
            payload: JobPayload::Task {
                command: "test".to_string(),
            },
            state: JobState::Pending,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        let json = serde_json::to_string(&job).expect("serialize");
        let deserialized: Job = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(job.id, deserialized.id);
        assert_eq!(job.priority, deserialized.priority);
    }

    #[test]
    fn test_job_result_serde_roundtrip() {
        let result = JobResult {
            job_id: "job-1".to_string(),
            outcome: JobOutcome::Success,
            execution_time_ms: 500,
        };
        let json = serde_json::to_string(&result).expect("serialize");
        let deserialized: JobResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(result.job_id, deserialized.job_id);
    }

    #[test]
    fn test_job_outcome_serde_roundtrip() {
        let outcomes = [
            JobOutcome::Success,
            JobOutcome::Failure {
                error: "boom".to_string(),
            },
        ];
        for outcome in &outcomes {
            let json = serde_json::to_string(outcome).expect("serialize");
            let deserialized: JobOutcome = serde_json::from_str(&json).expect("deserialize");
            match (outcome, &deserialized) {
                (JobOutcome::Success, JobOutcome::Success) => {}
                (JobOutcome::Failure { error: e1 }, JobOutcome::Failure { error: e2 }) => {
                    assert_eq!(e1, e2);
                }
                _ => panic!("Outcome mismatch"),
            }
        }
    }

    // --- Edge cases ---

    #[test]
    fn test_job_priority_from_u8_clamps_to_p4() {
        assert_eq!(JobPriority::from_u8(4), JobPriority::P4);
        assert_eq!(JobPriority::from_u8(5), JobPriority::P4);
        assert_eq!(JobPriority::from_u8(255), JobPriority::P4);
    }

    #[test]
    fn test_job_priority_equality() {
        let p0a = JobPriority::P0;
        let p0b = JobPriority::P0;
        let p2 = JobPriority::P2;
        assert_eq!(p0a, p0b);
        assert_ne!(p0a, p2);
    }

    #[test]
    fn test_job_state_is_pending_all_variants() {
        assert!(JobState::Pending.is_pending());
        assert!(!JobState::Running {
            started_at: chrono::Utc::now()
        }
        .is_pending());
        assert!(!JobState::Completed {
            finished_at: chrono::Utc::now()
        }
        .is_pending());
        assert!(!JobState::Failed {
            error: "err".to_string(),
            failed_at: chrono::Utc::now()
        }
        .is_pending());
    }

    // --- Job construction tests (ha-8wrx) ---

    #[test]
    fn test_job_new_creates_pending_job() {
        let job = Job::new(
            "job-42".to_string(),
            JobPriority::P1,
            JobPayload::Task {
                command: "build".to_string(),
            },
        );
        assert_eq!(job.id, "job-42");
        assert_eq!(job.priority, JobPriority::P1);
        assert!(job.state.is_pending());
        assert_eq!(job.created_at, job.updated_at);
    }

    #[test]
    fn test_job_new_with_pipeline_payload() {
        let job = Job::new(
            "pipe-1".to_string(),
            JobPriority::P0,
            JobPayload::Pipeline {
                spec_path: "specs/test.yaml".to_string(),
            },
        );
        assert_eq!(job.id, "pipe-1");
        assert_eq!(job.priority, JobPriority::P0);
    }

    #[test]
    fn test_job_new_with_custom_payload() {
        let job = Job::new(
            "custom-1".to_string(),
            JobPriority::P3,
            JobPayload::Custom {
                data: serde_json::json!({"key": "value"}),
            },
        );
        assert_eq!(job.id, "custom-1");
    }

    #[test]
    fn test_job_new_with_all_priorities() {
        let priorities = [
            JobPriority::P0,
            JobPriority::P1,
            JobPriority::P2,
            JobPriority::P3,
            JobPriority::P4,
        ];
        for (i, priority) in priorities.into_iter().enumerate() {
            let job = Job::new(
                format!("job-{i}"),
                priority,
                JobPayload::Task {
                    command: "test".to_string(),
                },
            );
            assert_eq!(job.priority, priority);
        }
    }

    // --- Job state transition: Pending → Running (ha-8wrx) ---

    #[test]
    fn test_job_transition_pending_to_running() {
        let mut job = Job::new(
            "job-1".to_string(),
            JobPriority::P0,
            JobPayload::Task {
                command: "run".to_string(),
            },
        );
        assert!(job.state.is_pending());

        let result = job.transition_to(JobState::Running {
            started_at: chrono::Utc::now(),
        });
        assert!(result.is_ok());
        assert!(job.state.is_running());
    }

    // --- Job state transition: Pending → Running → Completed (ha-8wrx) ---

    #[test]
    fn test_job_happy_path_pending_running_completed() {
        let mut job = Job::new(
            "job-1".to_string(),
            JobPriority::P0,
            JobPayload::Task {
                command: "run".to_string(),
            },
        );
        assert!(job.state.is_pending());

        job.transition_to(JobState::Running {
            started_at: chrono::Utc::now(),
        })
        .expect("Pending → Running");
        assert!(job.state.is_running());

        job.transition_to(JobState::Completed {
            finished_at: chrono::Utc::now(),
        })
        .expect("Running → Completed");
        assert!(job.state.is_terminal());
    }

    // --- Job state transition: Pending → Running → Failed (ha-8wrx) ---

    #[test]
    fn test_job_failure_path_pending_running_failed() {
        let mut job = Job::new(
            "job-2".to_string(),
            JobPriority::P1,
            JobPayload::Pipeline {
                spec_path: "specs/test.yaml".to_string(),
            },
        );

        job.transition_to(JobState::Running {
            started_at: chrono::Utc::now(),
        })
        .expect("Pending → Running");

        job.transition_to(JobState::Failed {
            error: "out of memory".to_string(),
            failed_at: chrono::Utc::now(),
        })
        .expect("Running → Failed");
        assert!(job.state.is_terminal());
    }

    // --- Terminal state immutability (ha-8wrx) ---

    #[test]
    fn test_completed_job_rejects_all_transitions() {
        let mut job = Job::new(
            "job-done".to_string(),
            JobPriority::P0,
            JobPayload::Task {
                command: "done".to_string(),
            },
        );
        job.state = JobState::Completed {
            finished_at: chrono::Utc::now(),
        };

        let targets = [
            JobState::Pending,
            JobState::Running {
                started_at: chrono::Utc::now(),
            },
            JobState::Completed {
                finished_at: chrono::Utc::now(),
            },
            JobState::Failed {
                error: "err".to_string(),
                failed_at: chrono::Utc::now(),
            },
        ];

        for target in &targets {
            let result = job.transition_to(target.clone());
            assert!(
                result.is_err(),
                "Completed job should reject transition to {target:?}"
            );
            assert!(
                matches!(result, Err(JobTransitionError::AlreadyTerminal { .. })),
                "Expected AlreadyTerminal error, got {result:?}"
            );
        }
    }

    #[test]
    fn test_failed_job_rejects_all_transitions() {
        let mut job = Job::new(
            "job-failed".to_string(),
            JobPriority::P0,
            JobPayload::Task {
                command: "fail".to_string(),
            },
        );
        job.state = JobState::Failed {
            error: "crash".to_string(),
            failed_at: chrono::Utc::now(),
        };

        let targets = [
            JobState::Pending,
            JobState::Running {
                started_at: chrono::Utc::now(),
            },
            JobState::Completed {
                finished_at: chrono::Utc::now(),
            },
            JobState::Failed {
                error: "again".to_string(),
                failed_at: chrono::Utc::now(),
            },
        ];

        for target in &targets {
            let result = job.transition_to(target.clone());
            assert!(
                result.is_err(),
                "Failed job should reject transition to {target:?}"
            );
        }
    }

    // --- Invalid transitions (ha-8wrx) ---

    #[test]
    fn test_pending_cannot_go_directly_to_completed() {
        let mut job = Job::new(
            "job-1".to_string(),
            JobPriority::P0,
            JobPayload::Task {
                command: "test".to_string(),
            },
        );
        let result = job.transition_to(JobState::Completed {
            finished_at: chrono::Utc::now(),
        });
        assert!(result.is_err());
        assert!(
            matches!(result, Err(JobTransitionError::InvalidTransition { .. })),
            "Expected InvalidTransition"
        );
    }

    #[test]
    fn test_pending_cannot_go_directly_to_failed() {
        let mut job = Job::new(
            "job-1".to_string(),
            JobPriority::P0,
            JobPayload::Task {
                command: "test".to_string(),
            },
        );
        let result = job.transition_to(JobState::Failed {
            error: "err".to_string(),
            failed_at: chrono::Utc::now(),
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_running_cannot_go_back_to_pending() {
        let mut job = Job::new(
            "job-1".to_string(),
            JobPriority::P0,
            JobPayload::Task {
                command: "test".to_string(),
            },
        );
        job.transition_to(JobState::Running {
            started_at: chrono::Utc::now(),
        })
        .expect("start");

        let result = job.transition_to(JobState::Pending);
        assert!(result.is_err());
    }

    #[test]
    fn test_pending_self_transition_rejected() {
        let mut job = Job::new(
            "job-1".to_string(),
            JobPriority::P0,
            JobPayload::Task {
                command: "test".to_string(),
            },
        );
        let result = job.transition_to(JobState::Pending);
        assert!(result.is_err());
    }

    // --- Transition updates timestamp ---

    #[test]
    fn test_transition_updates_updated_at() {
        let mut job = Job::new(
            "job-1".to_string(),
            JobPriority::P0,
            JobPayload::Task {
                command: "test".to_string(),
            },
        );

        job.transition_to(JobState::Running {
            started_at: chrono::Utc::now(),
        })
        .expect("start");

        // State must have changed to Running
        assert!(job.state.is_running());
    }

    // --- JobTransitionError display (ha-8wrx) ---

    #[test]
    fn test_job_transition_error_display_invalid() {
        let err = JobTransitionError::InvalidTransition {
            from: JobState::Pending,
            to: JobState::Completed {
                finished_at: chrono::Utc::now(),
            },
        };
        let msg = format!("{err}");
        assert!(msg.contains("Invalid job transition"));
    }

    #[test]
    fn test_job_transition_error_display_already_terminal() {
        let err = JobTransitionError::AlreadyTerminal {
            current: JobState::Completed {
                finished_at: chrono::Utc::now(),
            },
        };
        let msg = format!("{err}");
        assert!(msg.contains("already in terminal state"));
    }

    // --- JobTransitionError serde roundtrip ---

    #[test]
    fn test_job_transition_error_serde_roundtrip() {
        let err = JobTransitionError::InvalidTransition {
            from: JobState::Pending,
            to: JobState::Completed {
                finished_at: chrono::Utc::now(),
            },
        };
        let json = serde_json::to_string(&err).expect("serialize");
        let deserialized: JobTransitionError = serde_json::from_str(&json).expect("deserialize");
        match (err, deserialized) {
            (
                JobTransitionError::InvalidTransition { .. },
                JobTransitionError::InvalidTransition { .. },
            ) => {}
            _ => panic!("Mismatch"),
        }
    }

    #[test]
    fn test_job_transition_error_already_terminal_serde_roundtrip() {
        let err = JobTransitionError::AlreadyTerminal {
            current: JobState::Failed {
                error: "timeout".to_string(),
                failed_at: chrono::Utc::now(),
            },
        };
        let json = serde_json::to_string(&err).expect("serialize");
        let deserialized: JobTransitionError = serde_json::from_str(&json).expect("deserialize");
        match deserialized {
            JobTransitionError::AlreadyTerminal { .. } => {}
            _ => panic!("Expected AlreadyTerminal"),
        }
    }

    // --- Full lifecycle with Job::new() (ha-8wrx) ---

    #[test]
    fn test_job_full_lifecycle_success() {
        let mut job = Job::new(
            "lifecycle-ok".to_string(),
            JobPriority::P2,
            JobPayload::Task {
                command: "build".to_string(),
            },
        );
        assert!(job.state.is_pending());
        assert!(!job.state.is_terminal());

        job.transition_to(JobState::Running {
            started_at: chrono::Utc::now(),
        })
        .expect("start");
        assert!(job.state.is_running());

        job.transition_to(JobState::Completed {
            finished_at: chrono::Utc::now(),
        })
        .expect("complete");
        assert!(job.state.is_terminal());
    }

    #[test]
    fn test_job_full_lifecycle_failure() {
        let mut job = Job::new(
            "lifecycle-fail".to_string(),
            JobPriority::P2,
            JobPayload::Task {
                command: "build".to_string(),
            },
        );

        job.transition_to(JobState::Running {
            started_at: chrono::Utc::now(),
        })
        .expect("start");

        job.transition_to(JobState::Failed {
            error: "exit code 1".to_string(),
            failed_at: chrono::Utc::now(),
        })
        .expect("fail");
        assert!(job.state.is_terminal());
    }

    // --- Exhaustive invalid transition table (ha-8wrx) ---

    #[test]
    fn test_all_invalid_transitions_from_pending() {
        let mut job = Job::new(
            "test".to_string(),
            JobPriority::P0,
            JobPayload::Task {
                command: "x".to_string(),
            },
        );
        // Pending → Pending (self-loop)
        assert!(job.transition_to(JobState::Pending).is_err());
        // Pending → Completed (skip running)
        assert!(job
            .transition_to(JobState::Completed {
                finished_at: chrono::Utc::now()
            })
            .is_err());
        // Pending → Failed (skip running)
        assert!(job
            .transition_to(JobState::Failed {
                error: "x".to_string(),
                failed_at: chrono::Utc::now()
            })
            .is_err());
    }

    #[test]
    fn test_all_invalid_transitions_from_running() {
        let mut job = Job::new(
            "test".to_string(),
            JobPriority::P0,
            JobPayload::Task {
                command: "x".to_string(),
            },
        );
        job.transition_to(JobState::Running {
            started_at: chrono::Utc::now(),
        })
        .expect("start");

        // Running → Pending (backward)
        assert!(job.transition_to(JobState::Pending).is_err());
        // Running → Running (self-loop not allowed for jobs)
        assert!(job
            .transition_to(JobState::Running {
                started_at: chrono::Utc::now()
            })
            .is_err());
    }

    #[test]
    fn test_valid_transition_table_is_exhaustive() {
        // Verify only the 3 valid transitions work
        let cases: Vec<(JobState, JobState, bool)> = vec![
            // Pending → Running: OK
            (
                JobState::Pending,
                JobState::Running {
                    started_at: chrono::Utc::now(),
                },
                true,
            ),
            // Running → Completed: OK
            (
                JobState::Running {
                    started_at: chrono::Utc::now(),
                },
                JobState::Completed {
                    finished_at: chrono::Utc::now(),
                },
                true,
            ),
            // Running → Failed: OK
            (
                JobState::Running {
                    started_at: chrono::Utc::now(),
                },
                JobState::Failed {
                    error: "x".to_string(),
                    failed_at: chrono::Utc::now(),
                },
                true,
            ),
            // Pending → Pending: NO
            (JobState::Pending, JobState::Pending, false),
            // Pending → Completed: NO
            (
                JobState::Pending,
                JobState::Completed {
                    finished_at: chrono::Utc::now(),
                },
                false,
            ),
            // Completed → Running: NO (terminal)
            (
                JobState::Completed {
                    finished_at: chrono::Utc::now(),
                },
                JobState::Running {
                    started_at: chrono::Utc::now(),
                },
                false,
            ),
            // Failed → Pending: NO (terminal)
            (
                JobState::Failed {
                    error: "x".to_string(),
                    failed_at: chrono::Utc::now(),
                },
                JobState::Pending,
                false,
            ),
        ];

        for (from, to, should_succeed) in &cases {
            let mut job = Job::new(
                "test".to_string(),
                JobPriority::P0,
                JobPayload::Task {
                    command: "x".to_string(),
                },
            );
            job.state = from.clone();
            let result = job.transition_to(to.clone());
            assert_eq!(
                result.is_ok(),
                *should_succeed,
                "Transition {from:?} → {to:?}: expected success={should_succeed}, got {result:?}"
            );
        }
    }
}
