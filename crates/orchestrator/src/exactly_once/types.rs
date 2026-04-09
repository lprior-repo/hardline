//! Domain types for exactly-once execution guarantee layer
//!
//! Core value objects and entities:
//! - IdempotencyKey: Uniquely identifies an operation for dedup
//! - Receipt: Proof that an operation completed successfully
//! - JournalEntry: Records operation intent for crash recovery
//! - OperationStatus: Lifecycle states for an operation

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IdempotencyKey(pub String);

impl IdempotencyKey {
    pub fn new(value: String) -> Result<Self, IdempotencyKeyError> {
        if value.is_empty() {
            return Err(IdempotencyKeyError::Empty);
        }
        if value.len() > 256 {
            return Err(IdempotencyKeyError::TooLong {
                len: value.len(),
                max: 256,
            });
        }
        Ok(Self(value))
    }

    #[must_use]
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    #[must_use]
    pub fn from_static(value: &str) -> Self {
        Self(value.to_string())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for IdempotencyKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IdempotencyKeyError {
    Empty,
    TooLong { len: usize, max: usize },
}

impl std::fmt::Display for IdempotencyKeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "IdempotencyKey cannot be empty"),
            Self::TooLong { len, max } => {
                write!(f, "IdempotencyKey too long: {len} chars (max {max})")
            }
        }
    }
}

impl std::error::Error for IdempotencyKeyError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationStatus {
    Intended,
    InProgress,
    Completed,
    Failed,
}

impl OperationStatus {
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    pub key: IdempotencyKey,
    pub completed_at: DateTime<Utc>,
    pub result_hash: Option<String>,
}

impl Receipt {
    pub fn new(key: IdempotencyKey) -> Self {
        Self {
            key,
            completed_at: Utc::now(),
            result_hash: None,
        }
    }

    pub fn with_hash(key: IdempotencyKey, result_hash: String) -> Self {
        Self {
            key,
            completed_at: Utc::now(),
            result_hash: Some(result_hash),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    pub key: IdempotencyKey,
    pub status: OperationStatus,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub error: Option<String>,
}

impl JournalEntry {
    pub fn new_intended(key: IdempotencyKey, payload: serde_json::Value) -> Self {
        let now = Utc::now();
        Self {
            key,
            status: OperationStatus::Intended,
            payload,
            created_at: now,
            updated_at: now,
            error: None,
        }
    }

    pub fn transition_to(
        &self,
        new_status: OperationStatus,
    ) -> Result<Self, JournalTransitionError> {
        match (&self.status, &new_status) {
            (OperationStatus::Intended, OperationStatus::InProgress) => {}
            (OperationStatus::InProgress, OperationStatus::Completed) => {}
            (OperationStatus::InProgress, OperationStatus::Failed) => {}
            (s, _) if s.is_terminal() => {
                return Err(JournalTransitionError::AlreadyTerminal { current: *s });
            }
            _ => {
                return Err(JournalTransitionError::InvalidTransition {
                    from: self.status,
                    to: new_status,
                });
            }
        }

        Ok(Self {
            key: self.key.clone(),
            status: new_status,
            payload: self.payload.clone(),
            created_at: self.created_at,
            updated_at: Utc::now(),
            error: None,
        })
    }

    pub fn with_error(&self, error: String) -> Self {
        Self {
            error: Some(error),
            ..self.clone()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JournalTransitionError {
    InvalidTransition {
        from: OperationStatus,
        to: OperationStatus,
    },
    AlreadyTerminal {
        current: OperationStatus,
    },
}

impl std::fmt::Display for JournalTransitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTransition { from, to } => {
                write!(f, "Invalid journal transition from {from:?} to {to:?}")
            }
            Self::AlreadyTerminal { current } => {
                write!(f, "Journal entry already terminal: {current:?}")
            }
        }
    }
}

impl std::error::Error for JournalTransitionError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_idempotency_key_new_valid() {
        let key = IdempotencyKey::new("op-123".to_string());
        assert!(key.is_ok());
        assert_eq!(key.as_ref().unwrap().as_str(), "op-123");
    }

    #[test]
    fn test_idempotency_key_new_empty_rejected() {
        let key = IdempotencyKey::new(String::new());
        assert!(key.is_err());
        assert_eq!(key.unwrap_err(), IdempotencyKeyError::Empty);
    }

    #[test]
    fn test_idempotency_key_new_too_long_rejected() {
        let long = "a".repeat(300);
        let key = IdempotencyKey::new(long);
        assert!(key.is_err());
        match key.unwrap_err() {
            IdempotencyKeyError::TooLong { len, max } => {
                assert_eq!(len, 300);
                assert_eq!(max, 256);
            }
            other => panic!("Expected TooLong, got {other:?}"),
        }
    }

    #[test]
    fn test_idempotency_key_generate_unique() {
        let a = IdempotencyKey::generate();
        let b = IdempotencyKey::generate();
        assert_ne!(a, b);
    }

    #[test]
    fn test_idempotency_key_display() {
        let key = IdempotencyKey::from_static("test-key");
        assert_eq!(format!("{key}"), "test-key");
    }

    #[test]
    fn test_operation_status_terminal() {
        assert!(!OperationStatus::Intended.is_terminal());
        assert!(!OperationStatus::InProgress.is_terminal());
        assert!(OperationStatus::Completed.is_terminal());
        assert!(OperationStatus::Failed.is_terminal());
    }

    #[test]
    fn test_receipt_new() {
        let key = IdempotencyKey::from_static("op-1");
        let receipt = Receipt::new(key.clone());
        assert_eq!(receipt.key, key);
        assert!(receipt.result_hash.is_none());
    }

    #[test]
    fn test_receipt_with_hash() {
        let key = IdempotencyKey::from_static("op-1");
        let receipt = Receipt::with_hash(key, "abc123".to_string());
        assert_eq!(receipt.result_hash.as_deref(), Some("abc123"));
    }

    #[test]
    fn test_journal_entry_new_intended() {
        let key = IdempotencyKey::from_static("op-1");
        let entry = JournalEntry::new_intended(key.clone(), serde_json::json!({"cmd": "merge"}));
        assert_eq!(entry.key, key);
        assert_eq!(entry.status, OperationStatus::Intended);
        assert_eq!(entry.created_at, entry.updated_at);
    }

    #[test]
    fn test_journal_entry_valid_transitions() {
        let key = IdempotencyKey::from_static("op-1");
        let entry = JournalEntry::new_intended(key, serde_json::json!({"cmd": "merge"}));

        let in_progress = entry
            .transition_to(OperationStatus::InProgress)
            .expect("intended -> in_progress");
        assert_eq!(in_progress.status, OperationStatus::InProgress);

        let completed = in_progress
            .transition_to(OperationStatus::Completed)
            .expect("in_progress -> completed");
        assert_eq!(completed.status, OperationStatus::Completed);
    }

    #[test]
    fn test_journal_entry_failed_transition() {
        let key = IdempotencyKey::from_static("op-1");
        let entry = JournalEntry::new_intended(key, serde_json::json!({"cmd": "merge"}));
        let in_progress = entry
            .transition_to(OperationStatus::InProgress)
            .expect("intended -> in_progress");
        let failed = in_progress
            .transition_to(OperationStatus::Failed)
            .expect("in_progress -> failed");
        assert_eq!(failed.status, OperationStatus::Failed);
    }

    #[test]
    fn test_journal_entry_terminal_rejects() {
        let key = IdempotencyKey::from_static("op-1");
        let entry = JournalEntry::new_intended(key, serde_json::json!({}));
        let completed = entry
            .transition_to(OperationStatus::InProgress)
            .and_then(|e| e.transition_to(OperationStatus::Completed))
            .expect("completed");

        let result = completed.transition_to(OperationStatus::InProgress);
        assert!(result.is_err());
    }

    #[test]
    fn test_journal_entry_with_error() {
        let key = IdempotencyKey::from_static("op-1");
        let entry = JournalEntry::new_intended(key, serde_json::json!({}));
        let failed = entry
            .transition_to(OperationStatus::InProgress)
            .and_then(|e| e.transition_to(OperationStatus::Failed))
            .expect("failed");
        let with_err = failed.with_error("disk full".to_string());
        assert_eq!(with_err.error.as_deref(), Some("disk full"));
    }

    #[test]
    fn test_idempotency_key_error_display() {
        let err = IdempotencyKeyError::Empty;
        assert!(err.to_string().contains("empty"));

        let err = IdempotencyKeyError::TooLong { len: 300, max: 256 };
        assert!(err.to_string().contains("300"));
    }

    #[test]
    fn test_journal_transition_error_display() {
        let err = JournalTransitionError::InvalidTransition {
            from: OperationStatus::Intended,
            to: OperationStatus::Completed,
        };
        assert!(err.to_string().contains("Invalid"));

        let err = JournalTransitionError::AlreadyTerminal {
            current: OperationStatus::Completed,
        };
        assert!(err.to_string().contains("terminal"));
    }

    #[test]
    fn test_serde_roundtrips() {
        let key = IdempotencyKey::from_static("serde-key");
        let json = serde_json::to_string(&key).expect("serialize key");
        let back: IdempotencyKey = serde_json::from_str(&json).expect("deserialize key");
        assert_eq!(key, back);

        let receipt = Receipt::with_hash(key, "hash123".to_string());
        let json = serde_json::to_string(&receipt).expect("serialize receipt");
        let back: Receipt = serde_json::from_str(&json).expect("deserialize receipt");
        assert_eq!(receipt.key, back.key);
        assert_eq!(receipt.result_hash, back.result_hash);

        let entry = JournalEntry::new_intended(
            IdempotencyKey::from_static("j-1"),
            serde_json::json!({"x": 1}),
        );
        let json = serde_json::to_string(&entry).expect("serialize entry");
        let back: JournalEntry = serde_json::from_str(&json).expect("deserialize entry");
        assert_eq!(entry.key, back.key);
        assert_eq!(entry.status, back.status);
    }

    #[test]
    fn test_idempotency_key_boundary_length() {
        let exactly_256 = "a".repeat(256);
        assert!(IdempotencyKey::new(exactly_256).is_ok());

        let at_257 = "a".repeat(257);
        assert!(IdempotencyKey::new(at_257).is_err());
    }

    #[test]
    fn test_intended_cannot_go_directly_to_completed() {
        let key = IdempotencyKey::from_static("skip");
        let entry = JournalEntry::new_intended(key, serde_json::json!({}));
        let result = entry.transition_to(OperationStatus::Completed);
        assert!(result.is_err());
    }

    #[test]
    fn test_intended_cannot_go_directly_to_failed() {
        let key = IdempotencyKey::from_static("skip-fail");
        let entry = JournalEntry::new_intended(key, serde_json::json!({}));
        let result = entry.transition_to(OperationStatus::Failed);
        assert!(result.is_err());
    }
}
