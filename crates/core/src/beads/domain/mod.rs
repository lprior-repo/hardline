#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! Domain types for the beads issue tracker.
//!
//! This module implements Domain-Driven Design principles:
//! - Semantic newtypes prevent primitive obsession
//! - Enum-based state makes illegal states unrepresentable
//! - Parse at boundaries, validate once
//! - Pure functional core, side effects at boundaries
//!
//! # Architecture
//!
//! - **Core types**: `IssueId`, `Title`, `Description` - validated newtypes
//! - **State types**: `IssueState` - closed state includes timestamp inline
//! - **Domain errors**: Structured errors with `thiserror`

mod collections;
mod errors;
mod ids;
mod priority;
mod state;
mod text;

#[cfg(test)]
mod tests;

// Re-exports for public API
pub use collections::{BlockedBy, DependsOn, Labels};
pub use errors::DomainError;
pub use ids::{Assignee, IssueId, ParentId};
pub use priority::Priority;
pub use state::{IssueState, IssueType};
pub use text::{Description, Title};
