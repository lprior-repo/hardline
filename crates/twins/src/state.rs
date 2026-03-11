#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! State management module for twin runtime
//!
//! Provides in-memory state tracking for requests and responses.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use im::Vector;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestRecord {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub method: String,
    pub path: String,
    pub request_headers: HashMap<String, String>,
    #[serde(default)]
    pub request_body: Option<String>,
    pub status: u16,
    pub response_headers: HashMap<String, String>,
    #[serde(default)]
    pub response_body: Option<String>,
}

impl RequestRecord {
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        method: String,
        path: String,
        request_headers: HashMap<String, String>,
        request_body: Option<String>,
        status: u16,
        response_headers: HashMap<String, String>,
        response_body: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            method,
            path,
            request_headers,
            request_body,
            status,
            response_headers,
            response_body,
        }
    }
}

pub trait TwinState: Default {
    #[must_use]
    fn add_record(&self, record: RequestRecord) -> Self;
    fn get_records(&self) -> Vector<RequestRecord>;
    fn record_count(&self) -> usize;
    #[must_use]
    fn clear(&self) -> Self;
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InMemoryTwinState {
    records: Vector<RequestRecord>,
}

impl InMemoryTwinState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            records: Vector::new(),
        }
    }
}

impl TwinState for InMemoryTwinState {
    fn add_record(&self, record: RequestRecord) -> Self {
        let mut new_records = self.records.clone();
        new_records.push_back(record);
        Self {
            records: new_records,
        }
    }

    fn get_records(&self) -> Vector<RequestRecord> {
        self.records.clone()
    }

    fn record_count(&self) -> usize {
        self.records.len()
    }

    fn clear(&self) -> Self {
        Self {
            records: Vector::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_record() {
        let state = InMemoryTwinState::new();
        let record = RequestRecord::new(
            "GET".to_string(),
            "/test".to_string(),
            HashMap::new(),
            None,
            200,
            HashMap::new(),
            None,
        );
        let new_state = state.add_record(record);
        assert_eq!(new_state.record_count(), 1);
    }

    #[test]
    fn test_clear() {
        let state = InMemoryTwinState::new();
        let record = RequestRecord::new(
            "GET".to_string(),
            "/test".to_string(),
            HashMap::new(),
            None,
            200,
            HashMap::new(),
            None,
        );
        let state_with_record = state.add_record(record);
        let cleared = state_with_record.clear();
        assert_eq!(cleared.record_count(), 0);
    }
}
