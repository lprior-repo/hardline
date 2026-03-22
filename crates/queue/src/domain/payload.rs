#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Payload(serde_json::Value);

impl Payload {
    pub fn new(value: serde_json::Value) -> Result<Self, JobCreationError> {
        Ok(Self(value))
    }

    pub fn from_str(s: &str) -> Result<Self, JobCreationError> {
        let value: serde_json::Value = serde_json::from_str(s)
            .map_err(|_| JobCreationError::InvalidPayload(PayloadError::MalformedJson))?;
        Ok(Self(value))
    }

    pub fn empty() -> Result<Self, JobCreationError> {
        Err(JobCreationError::InvalidPayload(PayloadError::Empty))
    }

    pub fn as_value(&self) -> &serde_json::Value {
        &self.0
    }

    pub fn into_inner(self) -> serde_json::Value {
        self.0
    }
}

impl std::fmt::Display for Payload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone)]
pub enum PayloadError {
    Empty,
    MalformedJson,
}

impl std::fmt::Display for PayloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "payload cannot be empty"),
            Self::MalformedJson => write!(f, "payload must be valid JSON"),
        }
    }
}

impl std::error::Error for PayloadError {}

use super::job_id::JobCreationError;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_valid_json() {
        let payload = Payload::from_str(r#"{"key":"value"}"#);
        assert!(payload.is_ok());
    }

    #[test]
    fn payload_malformed_json_rejected() {
        let payload = Payload::from_str("not json {{");
        assert!(payload.is_err());
    }

    #[test]
    fn payload_empty_rejected() {
        let payload = Payload::empty();
        assert!(payload.is_err());
    }
}
