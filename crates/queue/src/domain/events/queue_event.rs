use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueueEvent {
    EntryEnqueued {
        entry_id: String,
        session_id: String,
        priority: u8,
        timestamp: DateTime<Utc>,
    },
    EntryClaimed {
        entry_id: String,
        agent_id: String,
        timestamp: DateTime<Utc>,
    },
    RebaseStarted {
        entry_id: String,
        timestamp: DateTime<Utc>,
    },
    RebaseCompleted {
        entry_id: String,
        timestamp: DateTime<Utc>,
    },
    TestingStarted {
        entry_id: String,
        timestamp: DateTime<Utc>,
    },
    TestingCompleted {
        entry_id: String,
        success: bool,
        timestamp: DateTime<Utc>,
    },
    MergeReady {
        entry_id: String,
        timestamp: DateTime<Utc>,
    },
    MergeStarted {
        entry_id: String,
        timestamp: DateTime<Utc>,
    },
    MergeCompleted {
        entry_id: String,
        timestamp: DateTime<Utc>,
    },
    EntryRetried {
        entry_id: String,
        retry_count: u32,
        timestamp: DateTime<Utc>,
    },
    EntryCancelled {
        entry_id: String,
        reason: Option<String>,
        timestamp: DateTime<Utc>,
    },
    EntryFailed {
        entry_id: String,
        error: String,
        retryable: bool,
        timestamp: DateTime<Utc>,
    },
}

impl QueueEvent {
    pub fn entry_enqueued(entry_id: String, session_id: String, priority: u8) -> Self {
        Self::EntryEnqueued {
            entry_id,
            session_id,
            priority,
            timestamp: Utc::now(),
        }
    }

    pub fn entry_claimed(entry_id: String, agent_id: String) -> Self {
        Self::EntryClaimed {
            entry_id,
            agent_id,
            timestamp: Utc::now(),
        }
    }

    pub fn rebase_started(entry_id: String) -> Self {
        Self::RebaseStarted {
            entry_id,
            timestamp: Utc::now(),
        }
    }

    pub fn testing_completed(entry_id: String, success: bool) -> Self {
        Self::TestingCompleted {
            entry_id,
            success,
            timestamp: Utc::now(),
        }
    }

    pub fn entry_failed(entry_id: String, error: String, retryable: bool) -> Self {
        Self::EntryFailed {
            entry_id,
            error,
            retryable,
            timestamp: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_entry_enqueued_construction() {
        let event = QueueEvent::entry_enqueued("e1".into(), "s1".into(), 5);
        match event {
            QueueEvent::EntryEnqueued {
                entry_id,
                session_id,
                priority,
                ..
            } => {
                assert_eq!(entry_id, "e1");
                assert_eq!(session_id, "s1");
                assert_eq!(priority, 5);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn event_entry_claimed_construction() {
        let event = QueueEvent::entry_claimed("e1".into(), "agent-1".into());
        match event {
            QueueEvent::EntryClaimed {
                entry_id, agent_id, ..
            } => {
                assert_eq!(entry_id, "e1");
                assert_eq!(agent_id, "agent-1");
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn event_rebase_started_construction() {
        let event = QueueEvent::rebase_started("e1".into());
        match event {
            QueueEvent::RebaseStarted { entry_id, .. } => {
                assert_eq!(entry_id, "e1");
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn event_testing_completed_success() {
        let event = QueueEvent::testing_completed("e1".into(), true);
        match event {
            QueueEvent::TestingCompleted {
                entry_id, success, ..
            } => {
                assert_eq!(entry_id, "e1");
                assert!(success);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn event_testing_completed_failure() {
        let event = QueueEvent::testing_completed("e1".into(), false);
        match event {
            QueueEvent::TestingCompleted { success, .. } => {
                assert!(!success);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn event_entry_failed_retryable() {
        let event = QueueEvent::entry_failed("e1".into(), "timeout".into(), true);
        match event {
            QueueEvent::EntryFailed {
                error, retryable, ..
            } => {
                assert_eq!(error, "timeout");
                assert!(retryable);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn event_entry_failed_terminal() {
        let event = QueueEvent::entry_failed("e1".into(), "fatal".into(), false);
        match event {
            QueueEvent::EntryFailed { retryable, .. } => {
                assert!(!retryable);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn event_entry_enqueued_timestamp_is_recent() {
        let before = Utc::now();
        let event = QueueEvent::entry_enqueued("e1".into(), "s1".into(), 10);
        let after = Utc::now();
        match event {
            QueueEvent::EntryEnqueued { timestamp, .. } => {
                assert!(timestamp >= before);
                assert!(timestamp <= after);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn event_entry_enqueued_serde_roundtrip() {
        let event = QueueEvent::entry_enqueued("e1".into(), "s1".into(), 42);
        let json = serde_json::to_string(&event).unwrap();
        let back: QueueEvent = serde_json::from_str(&json).unwrap();
        match back {
            QueueEvent::EntryEnqueued { priority, .. } => {
                assert_eq!(priority, 42);
            }
            _ => panic!("Wrong variant after deserialization"),
        }
    }

    #[test]
    fn event_entry_failed_serde_roundtrip() {
        let event = QueueEvent::entry_failed("e1".into(), "err".into(), true);
        let json = serde_json::to_string(&event).unwrap();
        let back: QueueEvent = serde_json::from_str(&json).unwrap();
        match back {
            QueueEvent::EntryFailed { retryable, .. } => {
                assert!(retryable);
            }
            _ => panic!("Wrong variant after deserialization"),
        }
    }

    #[test]
    fn event_all_variants_debug() {
        let events = vec![
            QueueEvent::EntryEnqueued {
                entry_id: "e".into(),
                session_id: "s".into(),
                priority: 1,
                timestamp: Utc::now(),
            },
            QueueEvent::EntryClaimed {
                entry_id: "e".into(),
                agent_id: "a".into(),
                timestamp: Utc::now(),
            },
            QueueEvent::RebaseStarted {
                entry_id: "e".into(),
                timestamp: Utc::now(),
            },
            QueueEvent::RebaseCompleted {
                entry_id: "e".into(),
                timestamp: Utc::now(),
            },
            QueueEvent::TestingStarted {
                entry_id: "e".into(),
                timestamp: Utc::now(),
            },
            QueueEvent::TestingCompleted {
                entry_id: "e".into(),
                success: true,
                timestamp: Utc::now(),
            },
            QueueEvent::MergeReady {
                entry_id: "e".into(),
                timestamp: Utc::now(),
            },
            QueueEvent::MergeStarted {
                entry_id: "e".into(),
                timestamp: Utc::now(),
            },
            QueueEvent::MergeCompleted {
                entry_id: "e".into(),
                timestamp: Utc::now(),
            },
            QueueEvent::EntryRetried {
                entry_id: "e".into(),
                retry_count: 1,
                timestamp: Utc::now(),
            },
            QueueEvent::EntryCancelled {
                entry_id: "e".into(),
                reason: None,
                timestamp: Utc::now(),
            },
            QueueEvent::EntryFailed {
                entry_id: "e".into(),
                error: "err".into(),
                retryable: false,
                timestamp: Utc::now(),
            },
        ];
        for event in &events {
            let debug = format!("{event:?}");
            assert!(!debug.is_empty());
        }
    }

    #[test]
    fn event_entry_cancelled_with_reason() {
        let event = QueueEvent::EntryCancelled {
            entry_id: "e1".into(),
            reason: Some("user request".into()),
            timestamp: Utc::now(),
        };
        match event {
            QueueEvent::EntryCancelled { reason, .. } => {
                assert_eq!(reason.as_deref(), Some("user request"));
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn event_entry_retried() {
        let event = QueueEvent::EntryRetried {
            entry_id: "e1".into(),
            retry_count: 3,
            timestamp: Utc::now(),
        };
        match event {
            QueueEvent::EntryRetried {
                retry_count, ..
            } => {
                assert_eq!(retry_count, 3);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn event_clone() {
        let event = QueueEvent::entry_enqueued("e1".into(), "s1".into(), 10);
        let cloned = event.clone();
        let _ = cloned;
    }

    // --- Additional serde roundtrip tests for all variants ---

    #[test]
    fn event_entry_claimed_serde_roundtrip() {
        let event = QueueEvent::entry_claimed("e1".into(), "agent-1".into());
        let json = serde_json::to_string(&event).unwrap();
        let back: QueueEvent = serde_json::from_str(&json).unwrap();
        match back {
            QueueEvent::EntryClaimed { agent_id, .. } => {
                assert_eq!(agent_id, "agent-1");
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn event_rebase_started_serde_roundtrip() {
        let event = QueueEvent::rebase_started("e1".into());
        let json = serde_json::to_string(&event).unwrap();
        let back: QueueEvent = serde_json::from_str(&json).unwrap();
        match back {
            QueueEvent::RebaseStarted { entry_id, .. } => {
                assert_eq!(entry_id, "e1");
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn event_rebase_completed_serde_roundtrip() {
        let event = QueueEvent::RebaseCompleted {
            entry_id: "e1".into(),
            timestamp: Utc::now(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let back: QueueEvent = serde_json::from_str(&json).unwrap();
        match back {
            QueueEvent::RebaseCompleted { entry_id, .. } => {
                assert_eq!(entry_id, "e1");
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn event_testing_started_serde_roundtrip() {
        let event = QueueEvent::TestingStarted {
            entry_id: "e1".into(),
            timestamp: Utc::now(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let back: QueueEvent = serde_json::from_str(&json).unwrap();
        match back {
            QueueEvent::TestingStarted { entry_id, .. } => {
                assert_eq!(entry_id, "e1");
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn event_testing_completed_serde_roundtrip() {
        let event = QueueEvent::testing_completed("e1".into(), true);
        let json = serde_json::to_string(&event).unwrap();
        let back: QueueEvent = serde_json::from_str(&json).unwrap();
        match back {
            QueueEvent::TestingCompleted { entry_id, success, .. } => {
                assert_eq!(entry_id, "e1");
                assert!(success);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn event_merge_ready_serde_roundtrip() {
        let event = QueueEvent::MergeReady {
            entry_id: "e1".into(),
            timestamp: Utc::now(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let back: QueueEvent = serde_json::from_str(&json).unwrap();
        match back {
            QueueEvent::MergeReady { entry_id, .. } => {
                assert_eq!(entry_id, "e1");
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn event_merge_started_serde_roundtrip() {
        let event = QueueEvent::MergeStarted {
            entry_id: "e1".into(),
            timestamp: Utc::now(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let back: QueueEvent = serde_json::from_str(&json).unwrap();
        match back {
            QueueEvent::MergeStarted { entry_id, .. } => {
                assert_eq!(entry_id, "e1");
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn event_merge_completed_serde_roundtrip() {
        let event = QueueEvent::MergeCompleted {
            entry_id: "e1".into(),
            timestamp: Utc::now(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let back: QueueEvent = serde_json::from_str(&json).unwrap();
        match back {
            QueueEvent::MergeCompleted { entry_id, .. } => {
                assert_eq!(entry_id, "e1");
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn event_entry_retried_serde_roundtrip() {
        let event = QueueEvent::EntryRetried {
            entry_id: "e1".into(),
            retry_count: 5,
            timestamp: Utc::now(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let back: QueueEvent = serde_json::from_str(&json).unwrap();
        match back {
            QueueEvent::EntryRetried { retry_count, .. } => {
                assert_eq!(retry_count, 5);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn event_entry_cancelled_serde_roundtrip() {
        let event = QueueEvent::EntryCancelled {
            entry_id: "e1".into(),
            reason: Some("timeout".into()),
            timestamp: Utc::now(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let back: QueueEvent = serde_json::from_str(&json).unwrap();
        match back {
            QueueEvent::EntryCancelled { reason, .. } => {
                assert_eq!(reason.as_deref(), Some("timeout"));
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn event_entry_cancelled_none_reason_serde_roundtrip() {
        let event = QueueEvent::EntryCancelled {
            entry_id: "e1".into(),
            reason: None,
            timestamp: Utc::now(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let back: QueueEvent = serde_json::from_str(&json).unwrap();
        match back {
            QueueEvent::EntryCancelled { reason, .. } => {
                assert!(reason.is_none());
            }
            _ => panic!("Wrong variant"),
        }
    }

    // --- Timestamp ordering ---

    #[test]
    fn event_timestamps_are_reasonably_recent() {
        let before = Utc::now();
        let event = QueueEvent::entry_enqueued("e1".into(), "s1".into(), 10);
        let after = Utc::now();

        match &event {
            QueueEvent::EntryEnqueued { timestamp, .. } => {
                assert!(*timestamp >= before);
                assert!(*timestamp <= after);
            }
            _ => panic!("Wrong variant"),
        }
    }

    // --- Edge cases ---

    #[test]
    fn event_entry_retried_zero_retry_count() {
        let event = QueueEvent::EntryRetried {
            entry_id: "e1".into(),
            retry_count: 0,
            timestamp: Utc::now(),
        };
        match event {
            QueueEvent::EntryRetried { retry_count, .. } => {
                assert_eq!(retry_count, 0);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn event_entry_failed_empty_error_string() {
        let event = QueueEvent::entry_failed("e1".into(), "".into(), true);
        match event {
            QueueEvent::EntryFailed { error, .. } => {
                assert_eq!(error, "");
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn event_entry_enqueued_with_priority_zero() {
        let event = QueueEvent::entry_enqueued("e1".into(), "s1".into(), 0);
        match event {
            QueueEvent::EntryEnqueued { priority, .. } => {
                assert_eq!(priority, 0);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn event_entry_enqueued_with_priority_max() {
        let event = QueueEvent::entry_enqueued("e1".into(), "s1".into(), 255);
        match event {
            QueueEvent::EntryEnqueued { priority, .. } => {
                assert_eq!(priority, 255);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn event_entry_claimed_empty_strings() {
        let event = QueueEvent::entry_claimed("".into(), "".into());
        match event {
            QueueEvent::EntryClaimed { entry_id, agent_id, .. } => {
                assert_eq!(entry_id, "");
                assert_eq!(agent_id, "");
            }
            _ => panic!("Wrong variant"),
        }
    }
}
