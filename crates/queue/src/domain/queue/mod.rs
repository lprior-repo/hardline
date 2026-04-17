#![allow(clippy::module_inception)]
//! Queue domain module - Immutable persistent queue implementation
//!
//! This module provides an immutable, persistent queue using functional paradigms:
//! - All operations return new Queue instances (persistent data structure)
//! - Railway-Oriented Programming with `Result` return types
//! - Pure functions - no I/O, no side effects
//! - Domain validation errors (ValidationError)
//! - Functional patterns: iterators, combinators, no for loops

pub mod entry;
pub mod queue;
pub mod status;

#[cfg(test)]
mod tests;

// Re-export all public types for convenient access
pub use entry::QueueEntry;
pub use queue::Queue;
pub use status::{QueueStatus, MAX_PRIORITY};
