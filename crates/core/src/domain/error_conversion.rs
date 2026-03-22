//! Error conversion traits and implementations for the domain layer.
//!
//! This module provides comprehensive error conversion between domain error types,
//! improving ergonomics while maintaining error context. Following DDD principles,
//! errors are categorized and converted with clear preservation of information.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

// Re-export from sibling modules (which live in domain/ as error_conversion_*)
// These are accessible because domain/mod.rs declares pub mod error_conversion;
// which loads this file, and the sibling files are in the same directory

// Bring the implementation modules into scope via use
use crate::domain::error_conversion_context as context;
use crate::domain::error_conversion_extension as extension;

// Re-export public traits for ergonomic access
pub use context::IntoRepositoryError;
pub use extension::{AggregateErrorExt, IdentifierErrorExt};
