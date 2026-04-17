//! Event serialization utilities
//!
//! Provides functions for serializing and deserializing domain events
//! to/from JSON format.

use crate::domain::DomainEvent;

/// Serialize an event to JSON
///
/// # Errors
///
/// Returns an error if serialization fails
pub fn serialize_event(event: &DomainEvent) -> Result<String, serde_json::Error> {
    serde_json::to_string(event)
}

/// Deserialize an event from JSON
///
/// # Errors
///
/// Returns an error if deserialization fails
pub fn deserialize_event(json: &str) -> Result<DomainEvent, serde_json::Error> {
    serde_json::from_str(json)
}

/// Serialize an event to JSON bytes
///
/// # Errors
///
/// Returns an error if serialization fails
pub fn serialize_event_bytes(event: &DomainEvent) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(event)
}

/// Deserialize an event from JSON bytes
///
/// # Errors
///
/// Returns an error if deserialization fails
pub fn deserialize_event_bytes(bytes: &[u8]) -> Result<DomainEvent, serde_json::Error> {
    serde_json::from_slice(bytes)
}
