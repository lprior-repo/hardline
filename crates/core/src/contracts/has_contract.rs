//! Trait for types with contracts

// Re-export for trait use
pub use super::types::TypeContract;
use crate::Result;

// ═══════════════════════════════════════════════════════════════════════════
// TRAIT FOR TYPES WITH CONTRACTS
// ═══════════════════════════════════════════════════════════════════════════

/// Trait for types that have contracts
pub trait HasContract {
    /// Get the contract for this type
    fn contract() -> TypeContract;

    /// Validate an instance against its contract
    fn validate(&self) -> Result<()>;

    /// Get JSON Schema representation
    ///
    /// # Returns
    ///
    /// Returns the JSON schema for this contract type. The result should be used
    /// as this generates a complete schema definition.
    #[must_use]
    fn json_schema() -> serde_json::Value {
        Self::contract().to_json_schema()
    }
}
