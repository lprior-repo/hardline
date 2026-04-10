#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! State management module for twin runtime
//!
//! Provides in-memory state tracking for requests and responses.
//! Uses immutable data structures (`im::Vector`) for thread-safe,
//! copy-on-write state transitions.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use im::Vector;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A recorded HTTP request/response pair captured by the twin.
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

/// Filter criteria for querying request records.
#[derive(Debug, Clone, Default)]
pub struct RecordFilter {
    pub method: Option<String>,
    pub path: Option<String>,
    pub status: Option<u16>,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
}

impl RecordFilter {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn method(mut self, method: impl Into<String>) -> Self {
        self.method = Some(method.into());
        self
    }

    #[must_use]
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    #[must_use]
    pub const fn status(mut self, status: u16) -> Self {
        self.status = Some(status);
        self
    }

    #[must_use]
    pub const fn since(mut self, since: DateTime<Utc>) -> Self {
        self.since = Some(since);
        self
    }

    #[must_use]
    pub const fn until(mut self, until: DateTime<Utc>) -> Self {
        self.until = Some(until);
        self
    }
}

fn matches_filter(record: &RequestRecord, filter: &RecordFilter) -> bool {
    if let Some(ref method) = filter.method {
        if record.method != *method {
            return false;
        }
    }
    if let Some(ref path) = filter.path {
        if record.path != *path {
            return false;
        }
    }
    if let Some(status) = filter.status {
        if record.status != status {
            return false;
        }
    }
    if let Some(since) = filter.since {
        if record.timestamp < since {
            return false;
        }
    }
    if let Some(until) = filter.until {
        if record.timestamp > until {
            return false;
        }
    }
    true
}

/// Trait for twin state management.
///
/// All methods use immutable transitions — they take `&self` and return
/// a new `Self`, leaving the original unchanged.
pub trait TwinState: Default {
    #[must_use]
    fn add_record(&self, record: RequestRecord) -> Self;

    fn get_records(&self) -> Vector<RequestRecord>;

    fn record_count(&self) -> usize;

    #[must_use]
    fn clear(&self) -> Self;

    fn find_by_id(&self, id: &str) -> Option<RequestRecord>;

    fn filter_records(&self, filter: &RecordFilter) -> Vector<RequestRecord>;

    #[must_use]
    fn remove_record(&self, id: &str) -> Self;

    fn records_by_method(&self, method: &str) -> Vector<RequestRecord>;

    fn records_by_path(&self, path: &str) -> Vector<RequestRecord>;

    fn records_by_status(&self, status: u16) -> Vector<RequestRecord>;
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

    fn find_by_id(&self, id: &str) -> Option<RequestRecord> {
        self.records.iter().find(|r| r.id == id).cloned()
    }

    fn filter_records(&self, filter: &RecordFilter) -> Vector<RequestRecord> {
        self.records
            .iter()
            .filter(|r| matches_filter(r, filter))
            .cloned()
            .collect()
    }

    fn remove_record(&self, id: &str) -> Self {
        let new_records: Vector<RequestRecord> = self
            .records
            .iter()
            .filter(|r| r.id != id)
            .cloned()
            .collect();
        Self {
            records: new_records,
        }
    }

    fn records_by_method(&self, method: &str) -> Vector<RequestRecord> {
        self.records
            .iter()
            .filter(|r| r.method == method)
            .cloned()
            .collect()
    }

    fn records_by_path(&self, path: &str) -> Vector<RequestRecord> {
        self.records
            .iter()
            .filter(|r| r.path == path)
            .cloned()
            .collect()
    }

    fn records_by_status(&self, status: u16) -> Vector<RequestRecord> {
        self.records
            .iter()
            .filter(|r| r.status == status)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_record(method: &str, path: &str, status: u16) -> RequestRecord {
        RequestRecord::new(
            method.to_string(),
            path.to_string(),
            HashMap::new(),
            None,
            status,
            HashMap::new(),
            None,
        )
    }

    fn sample_record_with_body(
        method: &str,
        path: &str,
        status: u16,
        response_body: &str,
    ) -> RequestRecord {
        RequestRecord::new(
            method.to_string(),
            path.to_string(),
            HashMap::new(),
            None,
            status,
            HashMap::new(),
            Some(response_body.to_string()),
        )
    }

    #[test]
    fn test_add_record() {
        let state = InMemoryTwinState::new();
        let record = sample_record("GET", "/test", 200);
        let new_state = state.add_record(record);
        assert_eq!(new_state.record_count(), 1);
        assert_eq!(state.record_count(), 0); // original unchanged
    }

    #[test]
    fn test_clear() {
        let state = InMemoryTwinState::new();
        let record = sample_record("GET", "/test", 200);
        let state_with_record = state.add_record(record);
        let cleared = state_with_record.clear();
        assert_eq!(cleared.record_count(), 0);
        assert_eq!(state_with_record.record_count(), 1); // original unchanged
    }

    #[test]
    fn test_find_by_id() {
        let state = InMemoryTwinState::new();
        let record = sample_record("GET", "/test", 200);
        let id = record.id.clone();
        let state = state.add_record(record);

        let found = state.find_by_id(&id);
        assert!(found.is_some());
        assert_eq!(found.unwrap().path, "/test");

        let not_found = state.find_by_id("nonexistent");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_remove_record() {
        let state = InMemoryTwinState::new();
        let r1 = sample_record("GET", "/a", 200);
        let r2 = sample_record("POST", "/b", 201);
        let id_to_remove = r1.id.clone();
        let state = state.add_record(r1).add_record(r2);

        assert_eq!(state.record_count(), 2);
        let state = state.remove_record(&id_to_remove);
        assert_eq!(state.record_count(), 1);
        assert_eq!(state.get_records()[0].method, "POST");
    }

    #[test]
    fn test_remove_nonexistent_is_noop() {
        let state = InMemoryTwinState::new();
        let record = sample_record("GET", "/test", 200);
        let state = state.add_record(record);
        let state = state.remove_record("does-not-exist");
        assert_eq!(state.record_count(), 1);
    }

    #[test]
    fn test_filter_by_method() {
        let state = InMemoryTwinState::new();
        let state = state
            .add_record(sample_record("GET", "/a", 200))
            .add_record(sample_record("POST", "/b", 201))
            .add_record(sample_record("GET", "/c", 200));

        let filter = RecordFilter::new().method("GET");
        let results = state.filter_records(&filter);
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.method == "GET"));
    }

    #[test]
    fn test_filter_by_path() {
        let state = InMemoryTwinState::new();
        let state = state
            .add_record(sample_record("GET", "/api/users", 200))
            .add_record(sample_record("POST", "/api/users", 201))
            .add_record(sample_record("GET", "/api/posts", 200));

        let filter = RecordFilter::new().path("/api/users");
        let results = state.filter_records(&filter);
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.path == "/api/users"));
    }

    #[test]
    fn test_filter_by_status() {
        let state = InMemoryTwinState::new();
        let state = state
            .add_record(sample_record("GET", "/a", 200))
            .add_record(sample_record("POST", "/b", 404))
            .add_record(sample_record("GET", "/c", 200));

        let filter = RecordFilter::new().status(200);
        let results = state.filter_records(&filter);
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.status == 200));
    }

    #[test]
    fn test_filter_by_time_range() {
        let state = InMemoryTwinState::new();
        let r1 = sample_record("GET", "/a", 200);
        let timestamp = r1.timestamp;
        let state = state.add_record(r1);

        let after = timestamp;
        let before = timestamp;
        let filter = RecordFilter::new().since(after).until(before);
        let results = state.filter_records(&filter);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_filter_combined() {
        let state = InMemoryTwinState::new();
        let state = state
            .add_record(sample_record("GET", "/api/users", 200))
            .add_record(sample_record("POST", "/api/users", 201))
            .add_record(sample_record("GET", "/api/users", 404))
            .add_record(sample_record("GET", "/api/posts", 200));

        let filter = RecordFilter::new()
            .method("GET")
            .path("/api/users")
            .status(200);
        let results = state.filter_records(&filter);
        assert_eq!(results.len(), 1);
        let r = &results[0];
        assert_eq!(r.method, "GET");
        assert_eq!(r.path, "/api/users");
        assert_eq!(r.status, 200);
    }

    #[test]
    fn test_filter_empty() {
        let state = InMemoryTwinState::new();
        let filter = RecordFilter::new().method("GET");
        let results = state.filter_records(&filter);
        assert!(results.is_empty());
    }

    #[test]
    fn test_records_by_method() {
        let state = InMemoryTwinState::new();
        let state = state
            .add_record(sample_record("GET", "/a", 200))
            .add_record(sample_record("POST", "/b", 201))
            .add_record(sample_record("GET", "/c", 200));

        let gets = state.records_by_method("GET");
        assert_eq!(gets.len(), 2);
        let posts = state.records_by_method("POST");
        assert_eq!(posts.len(), 1);
        let deletes = state.records_by_method("DELETE");
        assert!(deletes.is_empty());
    }

    #[test]
    fn test_records_by_path() {
        let state = InMemoryTwinState::new();
        let state = state
            .add_record(sample_record("GET", "/a", 200))
            .add_record(sample_record("POST", "/b", 201));

        let results = state.records_by_path("/a");
        assert_eq!(results.len(), 1);
        let empty = state.records_by_path("/z");
        assert!(empty.is_empty());
    }

    #[test]
    fn test_records_by_status() {
        let state = InMemoryTwinState::new();
        let state = state
            .add_record(sample_record("GET", "/a", 200))
            .add_record(sample_record("POST", "/b", 404));

        let ok = state.records_by_status(200);
        assert_eq!(ok.len(), 1);
        let not_found = state.records_by_status(404);
        assert_eq!(not_found.len(), 1);
        let server_error = state.records_by_status(500);
        assert!(server_error.is_empty());
    }

    #[test]
    fn test_record_with_response_body() {
        let record = sample_record_with_body("POST", "/api/send", 200, r#"{"sent":true}"#);
        assert_eq!(record.response_body, Some(r#"{"sent":true}"#.to_string()));
        assert!(record.request_body.is_none());
    }

    #[test]
    fn test_immutability_original_unchanged() {
        let state = InMemoryTwinState::new();
        let state_a = state.add_record(sample_record("GET", "/a", 200));
        let _state_b = state_a.add_record(sample_record("POST", "/b", 201));

        // state_a should still have only 1 record
        assert_eq!(state_a.record_count(), 1);
    }

    #[test]
    fn test_default_trait() {
        let state = InMemoryTwinState::default();
        assert_eq!(state.record_count(), 0);
    }
}
