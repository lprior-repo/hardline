//! Type contracts and validation system
//!
//! This module provides rich type information for AI-first design:
//! - Constraints (min/max, regex patterns)
//! - Contextual hints (examples, suggestions)
//! - Dependencies between fields
//! - Machine-readable schemas

pub mod builders;
pub mod has_contract;
pub mod impl_helpers;
pub mod tests;
pub mod types;

// Re-export all public types for convenience
pub use builders::{FieldContractBuilder, TypeContractBuilder};
pub use has_contract::HasContract;
pub use types::{Constraint, ContextualHint, FieldContract, HintType, TypeContract};
