//! Revert command handler - Revert a specific session merge.
//!
//! This handler ports the revert command from the isolate project,
//! adapted to hardline's Git-only architecture (no JJ).
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data** (`data.rs`): RevertOptions, RevertOutput, UndoEntry (inert, serializable)
//! - **Executor** (`executor.rs`): Re-exports GitExecutor from done handler
//! - **Actions** (`actions.rs`): run_revert, history reading, git reset (I/O operations)
//!
//! # Git vs JJ
//!
//! The original isolate revert used `jj rebase -d <pre_merge_commit>`.
//! This hardline version uses `git reset --hard <pre_merge_commit>` instead.
//!
//! # CLI Usage
//!
//! ```text
//! scp workspace revert feature-x           # Revert specific session merge
//! scp workspace revert --dry-run feat      # Preview revert
//! ```

pub mod actions;
pub mod data;
pub mod executor;

// Re-export public API
pub use actions::run_revert;
pub use data::{RevertOptions, RevertOutput};
