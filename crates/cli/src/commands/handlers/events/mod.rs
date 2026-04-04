//! Events command handler - Event streaming for multi-agent coordination.
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data** (`data.rs`): EventsOptions, EventType, EventEntry, EventsOutput
//!   (inert, serializable) and pure filter helper `event_type_matches`.
//! - **Actions** (`actions.rs`): run_events (I/O operations)
//!
//! # CLI Usage
//!
//! ```text
//! scp events                           # Show recent events
//! scp events --session feature-auth    # Filter by session
//! scp events --type session_created    # Filter by event type
//! scp events --follow                  # Stream events
//! scp events --limit 10               # Limit results
//! scp events --since 2025-01-01T00:00:00Z  # Since timestamp
//! ```

pub mod actions;
pub mod data;

pub use actions::run_events;
pub use data::{EventEntry, EventsOptions, EventsOutput, EventType};
