#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

//! Domain layer - Pure functional core with value objects
//!
//! This module contains domain entities and value objects that:
//! - Have validation on construction
//! - Are immutable
//! - Encode business rules in the type system
//! - Never perform I/O
//! - Return Result for all fallible operations

pub mod queue;
pub mod identifiers;
pub mod validation;

pub use queue::{Queue, QueueEntry, QueueStatus, MAX_PRIORITY};
pub use identifiers::{QueueEntryId, SessionName};
pub use validation::{ValidationError, ValidationResult};
