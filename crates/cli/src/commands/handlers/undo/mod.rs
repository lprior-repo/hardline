//! Undo command handler - Revert the most recent session merge.
//!
//! This handler ports the undo command from the isolate project,
//! adapted to hardline's Git-only architecture (no JJ).
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data** (`data.rs`): UndoOptions, UndoOutput, UndoEntry, UndoHistoryEntry,
//!   UndoHistoryOutput (inert, serializable)
//! - **Actions** (`actions.rs`): run_undo, run_list, history reading, git reset (I/O operations)
//!
//! # Git vs JJ
//!
//! The original isolate undo used `jj rebase -d <pre_merge_commit>`.
//! This hardline version uses `git reset --hard <pre_merge_commit>` instead.
//!
//! # CLI Usage
//!
//! ```text
//! scp undo                   # Undo the most recent session merge
//! scp undo --dry-run         # Preview undo without executing
//! scp undo --list            # Show undo history
//! ```

pub mod actions;
pub mod data;

// Re-export public API
pub use actions::run_undo;
pub use data::{UndoOptions, UndoOutput};
