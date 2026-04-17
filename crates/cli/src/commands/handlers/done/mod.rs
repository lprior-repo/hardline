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
//! - **VCS Ops** (`vcs_ops.rs`): workspace resolution, file/commit introspection,
//!   undo history, WorkspaceGitExecutor wrapper
//! - **Conflict** (`conflict.rs`): conflict-detection-only mode
//! - **Merge** (`merge.rs`): dry-run preview and the full done workflow
//! - **Actions** (`actions.rs`): `run_done` entry point (orchestrator)
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
pub mod conflict;
pub mod data;
pub mod executor;
pub mod merge;
pub mod vcs_ops;

#[cfg(test)]
pub(crate) mod test_support;

// Re-export public API
pub use actions::run_done;
pub use data::{DoneOptions, DoneOutput};
