//! Validated metadata type
//!
//! # Design Principles
//!
//! Following Scott Wlaschin's DDD principles:
//! - **Parse at boundaries** - Wraps `serde_json::Value` for type safety
//! - **Make illegal states unrepresentable** - Newtype distinction from arbitrary JSON

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// Validated metadata for extensibility
///
/// Wraps `serde_json::Value` to ensure metadata is valid JSON.
/// Unlike raw `serde_json::Value`, this provides:
/// - Type-level distinction from arbitrary JSON
/// - Clear intent for metadata usage
/// - Future extensibility for validation rules
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ValidatedMetadata(serde_json::Value);

impl ValidatedMetadata {
    /// Create new validated metadata from a JSON value
    ///
    /// Always succeeds since any `serde_json::Value` is valid.
    #[must_use]
    pub const fn new(value: serde_json::Value) -> Self {
        Self(value)
    }

    /// Create empty metadata (null value)
    #[must_use]
    pub const fn empty() -> Self {
        Self(serde_json::Value::Null)
    }

    /// Create metadata from an object
    #[must_use]
    pub const fn from_object(obj: serde_json::Map<String, serde_json::Value>) -> Self {
        Self(serde_json::Value::Object(obj))
    }

    /// Get the underlying JSON value
    #[must_use]
    pub const fn as_value(&self) -> &serde_json::Value {
        &self.0
    }

    /// Check if metadata is empty (null)
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        matches!(self.0, serde_json::Value::Null)
    }

    /// Get a field from the metadata
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.0.get(key)
    }

    /// Convert into the underlying JSON value
    #[must_use]
    pub fn into_value(self) -> serde_json::Value {
        self.0
    }
}

impl Default for ValidatedMetadata {
    fn default() -> Self {
        Self::empty()
    }
}

impl From<serde_json::Value> for ValidatedMetadata {
    fn from(value: serde_json::Value) -> Self {
        Self(value)
    }
}

impl From<ValidatedMetadata> for serde_json::Value {
    fn from(metadata: ValidatedMetadata) -> Self {
        metadata.0
    }
}
