//! Done command handler - Complete work and merge workspace to main.
//!
//! This handler ports the done/complete work command from the hardline project,
//! adapted to hardline's architecture and error handling.
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data** (`data.rs`): DoneOptions, DoneOutput, DonePreview, ConflictDetectionResult,
//!   CommitInfo, UndoEntry, DonePhase (inert, serializable)
//! - **Executor** (`executor.rs`): JjExecutor trait, RealJjExecutor, conflict detection
//!   (dependency injection for Git commands)
//! - **Actions** (`actions.rs`): run_done, execute_done_workflow (I/O operations)
//!
//! # Features (ported from hardline)
//!
//! - Conflict detection before merging
//! - Dry-run preview mode
//! - Undo history logging
//! - Squash support (future)
//! - Bead status update (future)
//! - Session state update (future)
//!
//! # CLI Usage
//!
//! ```text
//! scp workspace done                    # Complete current workspace
//! scp workspace done feature-x          # Complete specific workspace
//! scp workspace done --detect-conflicts # Check for conflicts only
//! scp workspace done --dry-run          # Preview without executing
//! scp workspace done --keep-workspace   # Keep workspace after merge
//! ```

pub mod actions;
pub mod data;
pub mod executor;

#[cfg(test)]
mod adversarial_tests;

#[cfg(test)]
mod integration_tests;

// Re-export public API
pub use actions::run_done;
pub use data::{DoneOptions, DoneOutput};
