#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

//! Queue use cases - Pure orchestration for queue operations
//!
//! This module implements Railway-Oriented Programming where each use case:
//! 1. Takes domain types as input
//! 2. Validates inputs using the validation layer
//! 3. Applies business logic
//! 4. Returns Result<T, DomainError>
//!
//! All functions are pure - no I/O, no side effects.

pub mod queue_use_cases;

pub use queue_use_cases::{
    DomainError, QueueEntryView, UseCaseResult, dequeue_session, enqueue_session,
    insert_at_position, list_queue, remove_at_position,
};
