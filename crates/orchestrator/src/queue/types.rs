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
}
