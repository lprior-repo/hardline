#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
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

    pub fn as_str(&self) -> &str {
        &self.0
    }

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

    pub fn as_str(&self) -> &str {
        &self.0
    }

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
            Self::InvalidId(msg) => write!(f, "Invalid ID: {}", msg),
            Self::InvalidPayload(e) => write!(f, "Invalid payload: {}", e),
            Self::InvalidPriority(v) => write!(f, "Invalid priority: {}", v),
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
}
