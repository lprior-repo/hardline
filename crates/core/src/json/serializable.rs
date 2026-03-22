//! JSON serialization trait

use serde::Serialize;

/// Trait for types that can be serialized to JSON
pub trait JsonSerializable: Serialize {
    /// Convert to pretty-printed JSON string
    fn to_json(&self) -> crate::error::Result<String> {
        serde_json::to_string_pretty(self).map_err(|e| crate::error::Error::JsonParse(e))
    }
}

// Implement for all Serialize types
impl<T: Serialize> JsonSerializable for T {}
