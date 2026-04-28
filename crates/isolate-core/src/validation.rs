//! Domain validation layer - Pure functions for business rule enforcement
//!
//! This module re-exports validation types from the domain layer.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

pub mod domain;
pub mod infrastructure;
// Note: validators module is incomplete and uses unstable features

// Re-export IdentifierError for convenience
pub use crate::domain::identifiers::IdentifierError;
