//! Wait command handler - blocking primitives for AI agents.
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data** (`data.rs`): WaitOptions, WaitCondition, WaitOutput
//!   (inert, serializable)
//! - **Actions** (`actions.rs`): run_wait (I/O operations)
//!
//! # CLI Usage
//!
//! ```text
//! scp wait session-exists <name>       # Wait for session to exist
//! scp wait session-unlocked <name>     # Wait for session to be unlocked
//! scp wait healthy                     # Wait for system to be healthy
//! scp wait session-status <name> <status>  # Wait for session status
//! ```

pub mod actions;
pub mod data;

pub use actions::run_wait;
pub use data::{format_condition, parse_condition, WaitCondition, WaitOptions, WaitOutput};
