//! Domain events module
//!
//! This module implements Domain-Driven Design event sourcing patterns.
//! Domain events represent important business events that have occurred in the system.
//!
//! # Design Principles
//!
//! - **Immutable**: Events cannot be modified after creation
//! - **Serializable**: All events can be serialized for persistence and transmission
//! - **Typed**: Each event carries specific, validated domain data
//! - **Timestamped**: All events include when they occurred
//! - **Pure**: Event creation is deterministic and side-effect free
//!
//! # Usage
//!
//! ```rust,ignore
//! // DomainEvent requires additional domain types for construction.
//! // See DomainEvent::session_created() and related constructors.
//! ```

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

mod bead_events;
mod domain_event;
#[cfg(test)]
mod domain_event_tests;
mod event_metadata;
mod serialization;
mod session_events;
mod workspace_events;

// Re-export the main domain event enum
// Re-export event types
pub use bead_events::{BeadClosedEvent, BeadCreatedEvent};
pub use domain_event::DomainEvent;
pub use event_metadata::{EventMetadata, StoredEvent};
// Re-export serialization functions
pub use serialization::{
    deserialize_event, deserialize_event_bytes, serialize_event, serialize_event_bytes,
};
pub use session_events::{SessionCompletedEvent, SessionCreatedEvent, SessionFailedEvent};
pub use workspace_events::{WorkspaceCreatedEvent, WorkspaceRemovedEvent};
