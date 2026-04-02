#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

use crate::domain::payload::PayloadError;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct JobId(String);

impl JobId {
    pub fn new(id: impl Into<String>) -> Result<Self, JobCreationError> {
        let id = id.into();
        if id.trim().is_empty() {
            Err(JobCreationError::InvalidId("JobId cannot be empty".into()))
        } else {
            Ok(Self(id))
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl std::fmt::Display for JobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct QueueId(String);

impl QueueId {
    pub fn new(id: impl Into<String>) -> Result<Self, JobCreationError> {
        let id = id.into();
        if id.trim().is_empty() {
            Err(JobCreationError::InvalidId(
                "QueueId cannot be empty".into(),
            ))
        } else {
            Ok(Self(id))
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl std::fmt::Display for QueueId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone)]
pub enum JobCreationError {
    InvalidId(String),
    InvalidPayload(PayloadError),
    InvalidPriority(u8),
}

impl std::fmt::Display for JobCreationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidId(msg) => write!(f, "Invalid ID: {msg}"),
            Self::InvalidPayload(e) => write!(f, "Invalid payload: {e}"),
            Self::InvalidPriority(v) => write!(f, "Invalid priority: {v}"),
        }
    }
}

impl std::error::Error for JobCreationError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_id_valid() {
        let id = JobId::new("job-1");
        assert!(id.is_ok());
        assert_eq!(id.unwrap().as_str(), "job-1");
    }

    #[test]
    fn job_id_empty_rejected() {
        let id = JobId::new("");
        assert!(id.is_err());
    }

    #[test]
    fn job_id_whitespace_rejected() {
        let id = JobId::new("   ");
        assert!(id.is_err());
    }

    #[test]
    fn queue_id_valid() {
        let id = QueueId::new("queue-1");
        assert!(id.is_ok());
    }

    #[test]
    fn queue_id_empty_rejected() {
        let id = QueueId::new("");
        assert!(id.is_err());
    }

    #[test]
    fn job_id_display() {
        let id = JobId::new("job-42").unwrap();
        assert_eq!(format!("{id}"), "job-42");
    }

    #[test]
    fn queue_id_display() {
        let id = QueueId::new("queue-7").unwrap();
        assert_eq!(format!("{id}"), "queue-7");
    }

    #[test]
    fn job_id_into_inner() {
        let id = JobId::new("my-job").unwrap();
        assert_eq!(id.into_inner(), "my-job");
    }

    #[test]
    fn queue_id_into_inner() {
        let id = QueueId::new("my-queue").unwrap();
        assert_eq!(id.into_inner(), "my-queue");
    }

    #[test]
    fn job_id_clone_and_eq() {
        let a = JobId::new("same").unwrap();
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn queue_id_clone_and_eq() {
        let a = QueueId::new("same").unwrap();
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn job_id_hash_consistency() {
        use std::collections::HashSet;
        let id = JobId::new("hash-test").unwrap();
        let mut set = HashSet::new();
        set.insert(id.clone());
        assert!(set.contains(&id));
    }

    #[test]
    fn queue_id_hash_consistency() {
        use std::collections::HashSet;
        let id = QueueId::new("hash-test").unwrap();
        let mut set = HashSet::new();
        set.insert(id.clone());
        assert!(set.contains(&id));
    }

    #[test]
    fn job_id_whitespace_only_rejected() {
        assert!(JobId::new("  \t").is_err());
    }

    #[test]
    fn queue_id_whitespace_only_rejected() {
        assert!(QueueId::new("  \t").is_err());
    }

    #[test]
    fn job_id_with_internal_spaces_accepted() {
        let id = JobId::new("job with spaces");
        assert!(id.is_ok());
        assert_eq!(id.unwrap().as_str(), "job with spaces");
    }

    #[test]
    fn job_creation_error_invalid_id_display() {
        let err = JobCreationError::InvalidId("bad".into());
        let msg = format!("{err}");
        assert!(msg.contains("bad"));
    }

    #[test]
    fn job_creation_error_invalid_priority_display() {
        let err = JobCreationError::InvalidPriority(42);
        let msg = format!("{err}");
        assert!(msg.contains("42"));
    }

    #[test]
    fn job_creation_error_implements_std_error() {
        let err: Box<dyn std::error::Error> = Box::new(JobCreationError::InvalidId("x".into()));
        let _ = format!("{err:?}");
    }

    #[test]
    fn job_id_serde_roundtrip() {
        let id = JobId::new("serde-test").unwrap();
        let json = serde_json::to_string(&id).unwrap();
        let back: JobId = serde_json::from_str(&json).unwrap();
        assert_eq!(back.as_str(), "serde-test");
    }

    #[test]
    fn queue_id_serde_roundtrip() {
        let id = QueueId::new("serde-queue").unwrap();
        let json = serde_json::to_string(&id).unwrap();
        let back: QueueId = serde_json::from_str(&json).unwrap();
        assert_eq!(back.as_str(), "serde-queue");
    }

    // --- Additional comprehensive tests ---

    #[test]
    fn job_id_new_with_unicode() {
        let id = JobId::new("job-unicode");
        assert!(id.is_ok());
    }

    #[test]
    fn queue_id_newline_tab_rejected() {
        assert!(QueueId::new("\n\t").is_err());
    }

    #[test]
    fn queue_id_with_internal_spaces_accepted() {
        let id = QueueId::new("queue with spaces");
        assert!(id.is_ok());
        assert_eq!(id.unwrap().as_str(), "queue with spaces");
    }

    #[test]
    fn job_id_new_with_tab_only() {
        assert!(JobId::new("\t").is_err());
    }

    #[test]
    fn job_id_new_with_newline_only() {
        assert!(JobId::new("\n").is_err());
    }

    #[test]
    fn job_id_single_char() {
        assert!(JobId::new("x").is_ok());
        assert_eq!(JobId::new("x").unwrap().as_str(), "x");
    }

    #[test]
    fn queue_id_single_char() {
        assert!(QueueId::new("q").is_ok());
        assert_eq!(QueueId::new("q").unwrap().as_str(), "q");
    }

    #[test]
    fn job_id_very_long_string() {
        let long = "a".repeat(10000);
        let id = JobId::new(&long);
        assert!(id.is_ok());
    }

    #[test]
    fn queue_id_very_long_string() {
        let long = "q".repeat(10000);
        let id = QueueId::new(&long);
        assert!(id.is_ok());
    }

    #[test]
    fn job_id_serde_roundtrip_empty_json_succeeds() {
        // Note: Serde Deserialize for JobId does not validate the inner string.
        // Validation only happens in JobId::new(). This tests the serde behavior.
        let result: Result<JobId, _> = serde_json::from_str("\"\"");
        assert!(result.is_ok());
    }

    #[test]
    fn queue_id_serde_roundtrip_empty_json_succeeds() {
        let result: Result<QueueId, _> = serde_json::from_str("\"\"");
        assert!(result.is_ok());
    }

    #[test]
    fn job_id_serde_roundtrip_whitespace_json_accepted() {
        let id = JobId::new("  spaced  ").unwrap();
        let json = serde_json::to_string(&id).unwrap();
        let back: JobId = serde_json::from_str(&json).unwrap();
        assert_eq!(back.as_str(), "  spaced  ");
    }

    #[test]
    fn job_id_equality_different_values() {
        let a = JobId::new("a").unwrap();
        let b = JobId::new("b").unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn queue_id_equality_different_values() {
        let a = QueueId::new("a").unwrap();
        let b = QueueId::new("b").unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn job_creation_error_invalid_payload_display() {
        use crate::domain::payload::PayloadError;
        let err = JobCreationError::InvalidPayload(PayloadError::MalformedJson);
        let msg = format!("{err}");
        assert!(msg.contains("Invalid payload"));
    }

    #[test]
    fn job_creation_error_clone() {
        let a = JobCreationError::InvalidId("test".into());
        let b = a.clone();
        let msg_a = format!("{a}");
        let msg_b = format!("{b}");
        assert_eq!(msg_a, msg_b);
    }

    #[test]
    fn job_creation_error_debug() {
        let err = JobCreationError::InvalidId("debug-test".into());
        let debug = format!("{err:?}");
        assert!(debug.contains("debug-test"));
    }

    #[test]
    fn queue_id_debug() {
        let id = QueueId::new("debug-queue").unwrap();
        let debug = format!("{id:?}");
        assert!(debug.contains("debug-queue"));
    }

    #[test]
    fn job_id_debug() {
        let id = JobId::new("debug-job").unwrap();
        let debug = format!("{id:?}");
        assert!(debug.contains("debug-job"));
    }

    // --- Proptests ---

    use proptest::prelude::*;
    use proptest::{prop_assert, prop_assert_eq};

    proptest! {
        #[test]
        fn proptest_job_id_roundtrip(
            input in "\\S.{0,99}"
        ) {
            let id = JobId::new(input.clone()).expect("valid input should parse");
            prop_assert_eq!(id.as_str(), input);
        }

        #[test]
        fn proptest_queue_id_roundtrip(
            input in "\\S.{0,99}"
        ) {
            let id = QueueId::new(input.clone()).expect("valid input should parse");
            prop_assert_eq!(id.as_str(), input);
        }

        #[test]
        fn proptest_job_id_serde_roundtrip(
            input in "\\S.{0,99}"
        ) {
            let id = JobId::new(input.clone()).expect("valid");
            let json = serde_json::to_string(&id).unwrap();
            let back: JobId = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(back.as_str(), input);
        }

        #[test]
        fn proptest_queue_id_serde_roundtrip(
            input in "\\S.{0,99}"
        ) {
            let id = QueueId::new(input.clone()).expect("valid");
            let json = serde_json::to_string(&id).unwrap();
            let back: QueueId = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(back.as_str(), input);
        }

        #[test]
        fn proptest_job_id_rejects_whitespace_only(
            input in "\\s+"
        ) {
            prop_assert!(JobId::new(input).is_err());
        }

        #[test]
        fn proptest_queue_id_rejects_whitespace_only(
            input in "\\s+"
        ) {
            prop_assert!(QueueId::new(input).is_err());
        }
    }
}
