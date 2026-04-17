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

    // ── Red Queen: adversarial state tests ──

    #[test]
    fn test_original_state_unchanged_after_add() {
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
        let _new_state = state.add_record(record);
        assert_eq!(
            state.record_count(),
            0,
            "Original state must not be mutated (persistence invariant)"
        );
    }

    #[test]
    fn test_original_state_unchanged_after_clear() {
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
        let populated = state.add_record(record);
        let _cleared = populated.clear();
        assert_eq!(
            populated.record_count(),
            1,
            "Original populated state must not be mutated by clear"
        );
    }

    #[test]
    fn test_add_multiple_records_preserves_order() {
        let state = InMemoryTwinState::new();
        let r1 = RequestRecord::new(
            "GET".to_string(),
            "/first".to_string(),
            HashMap::new(),
            None,
            200,
            HashMap::new(),
            None,
        );
        let r2 = RequestRecord::new(
            "POST".to_string(),
            "/second".to_string(),
            HashMap::new(),
            None,
            201,
            HashMap::new(),
            None,
        );
        let r3 = RequestRecord::new(
            "DELETE".to_string(),
            "/third".to_string(),
            HashMap::new(),
            None,
            204,
            HashMap::new(),
            None,
        );
        let state = state.add_record(r1).add_record(r2).add_record(r3);
        let records = state.get_records();
        assert_eq!(records.len(), 3);
        assert_eq!(records[0].path, "/first");
        assert_eq!(records[1].path, "/second");
        assert_eq!(records[2].path, "/third");
    }

    #[test]
    fn test_record_auto_generates_unique_id() {
        let r1 = RequestRecord::new(
            "GET".to_string(),
            "/a".to_string(),
            HashMap::new(),
            None,
            200,
            HashMap::new(),
            None,
        );
        let r2 = RequestRecord::new(
            "GET".to_string(),
            "/b".to_string(),
            HashMap::new(),
            None,
            200,
            HashMap::new(),
            None,
        );
        assert_ne!(r1.id, r2.id, "Each record must have a unique ID");
        assert!(!r1.id.is_empty());
    }

    #[test]
    fn test_record_auto_generates_timestamp() {
        let before = chrono::Utc::now();
        let record = RequestRecord::new(
            "GET".to_string(),
            "/test".to_string(),
            HashMap::new(),
            None,
            200,
            HashMap::new(),
            None,
        );
        let after = chrono::Utc::now();
        assert!(record.timestamp >= before);
        assert!(record.timestamp <= after);
    }

    #[test]
    fn test_record_with_body_preserves_content() {
        let body = Some(r#"{"key":"value"}"#.to_string());
        let record = RequestRecord::new(
            "POST".to_string(),
            "/api".to_string(),
            HashMap::new(),
            body.clone(),
            200,
            HashMap::new(),
            None,
        );
        assert_eq!(record.request_body, body);
    }

    #[test]
    fn test_record_with_headers_preserves_all() {
        let mut req_headers = HashMap::new();
        req_headers.insert("authorization".to_string(), "Bearer token".to_string());
        req_headers.insert("content-type".to_string(), "application/json".to_string());
        let mut resp_headers = HashMap::new();
        resp_headers.insert("x-request-id".to_string(), "abc-123".to_string());

        let record = RequestRecord::new(
            "POST".to_string(),
            "/api".to_string(),
            req_headers.clone(),
            None,
            201,
            resp_headers.clone(),
            None,
        );
        assert_eq!(record.request_headers, req_headers);
        assert_eq!(record.response_headers, resp_headers);
    }

    #[test]
    fn test_clear_and_repopulate_independently() {
        let state = InMemoryTwinState::new();
        let r1 = RequestRecord::new(
            "GET".to_string(),
            "/old".to_string(),
            HashMap::new(),
            None,
            200,
            HashMap::new(),
            None,
        );
        let populated = state.add_record(r1);
        let cleared = populated.clear();

        let r2 = RequestRecord::new(
            "POST".to_string(),
            "/new".to_string(),
            HashMap::new(),
            None,
            201,
            HashMap::new(),
            None,
        );
        let repopulated = cleared.add_record(r2);

        assert_eq!(repopulated.record_count(), 1);
        assert_eq!(repopulated.get_records()[0].path, "/new");
        // Original populated state still has old record
        assert_eq!(populated.record_count(), 1);
        assert_eq!(populated.get_records()[0].path, "/old");
    }

    #[test]
    fn test_default_is_empty() {
        let state = InMemoryTwinState::default();
        assert_eq!(state.record_count(), 0);
        assert!(state.get_records().is_empty());
    }

    #[test]
    fn test_get_records_returns_independent_clone() {
        let state = InMemoryTwinState::new();
        let r = RequestRecord::new(
            "GET".to_string(),
            "/test".to_string(),
            HashMap::new(),
            None,
            200,
            HashMap::new(),
            None,
        );
        let state = state.add_record(r);
        let mut records = state.get_records();
        // im::Vector clone should be independent
        records.clear();
        assert_eq!(
            state.record_count(),
            1,
            "Modifying get_records() clone must not affect state"
        );
    }

    #[test]
    fn test_request_record_serde_roundtrip() {
        let record = RequestRecord::new(
            "POST".to_string(),
            "/api/test".to_string(),
            HashMap::from([("x-custom".to_string(), "val".to_string())]),
            Some(r#"{"data":42}"#.to_string()),
            201,
            HashMap::from([("x-resp".to_string(), "ok".to_string())]),
            Some(r#"{"id":1}"#.to_string()),
        );
        let json = serde_json::to_string(&record).expect("serialize");
        let deserialized: RequestRecord = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(record.id, deserialized.id);
        assert_eq!(record.method, deserialized.method);
        assert_eq!(record.path, deserialized.path);
        assert_eq!(record.status, deserialized.status);
    }

    #[test]
    fn test_chained_add_returns_correct_count() {
        let state = InMemoryTwinState::new();
        let mut state = state;
        for i in 0..50 {
            let r = RequestRecord::new(
                "GET".to_string(),
                format!("/ep/{i}"),
                HashMap::new(),
                None,
                200,
                HashMap::new(),
                None,
            );
            state = state.add_record(r);
        }
        assert_eq!(state.record_count(), 50);
    }
}
