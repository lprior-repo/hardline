//! Queue module - Immutable persistent queue for merge queue entries
//!
//! This module provides an immutable, persistent queue using functional paradigms:
//! - All operations return new Queue instances (persistent data structure)
//! - Railway-Oriented Programming with `Result` return types
//! - Pure functions - no I/O, no side effects
//! - Domain validation errors (`ValidationError`)
//! - Functional patterns: iterators, combinators, no for loops

pub mod entry;
pub mod queue_impl;
pub mod status;
pub mod validation;

#[cfg(test)]
mod tests;

// Re-export types for convenience
pub use entry::QueueEntry;
pub use queue_impl::Queue;
pub use status::{QueueStatus, MAX_PRIORITY};
