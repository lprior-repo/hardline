//! Prune command handler - Remove invalid session records.
//!
//! Bulk cleanup primitive to remove all invalid session records in one
//! deterministic command. Invalid sessions are those where:
//! - The workspace directory no longer exists
//! - The session record exists in database but references missing paths
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data** (`data.rs`): PruneOptions, PruneMode, PruneOutput, PrunableItem (inert, serializable)
//! - **Actions** (`actions.rs`): run_prune (I/O operations)
//!
//! # CLI Usage
//!
//! ```text
//! scp prune                           # Show invalid sessions and prompt
//! scp prune --yes                     # Remove without confirmation
//! scp prune --dry-run                 # Show what would be removed
//! ```

pub mod actions;
pub mod data;

pub use actions::run_prune;
pub use data::{PrunableItem, PruneMode, PruneOptions, PruneOutput};
