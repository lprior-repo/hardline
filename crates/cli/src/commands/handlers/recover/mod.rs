//! Recover command handler - Auto-detect and fix common broken states.
//!
//! This handler ports the recover command from the isolate project,
//! adapted to hardline's architecture and pivoted from JJ to Git.
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data** (`data.rs`): RecoverOptions, RecoverOutput, RollbackOptions, RollbackOutput, Issue,
//!   RecoverPhase, compute_status, count_fixed, count_remaining (inert, serializable types + pure
//!   functions)
//! - **Actions** (`actions.rs`): run_recover, run_rollback (I/O operations)
//!
//! # Git-Based Recovery (no JJ)
//!
//! Recovery strategies use standard Git operations:
//! - `git worktree prune` to clean stale worktrees
//! - `git checkout` to fix detached HEAD
//! - `git reset --hard` for rollback operations
//! - `git ls-files --unmerged` for merge conflict detection
//! - `git symbolic-ref` for detached HEAD detection
//!
//! # CLI Usage
//!
//! ```text
//! scp recover                          # Auto-detect and fix Git/workspace issues
//! scp recover --diagnose               # Show issues without fixing
//! scp recover --target <session>       # Focus on a specific workspace
//! scp recover --dry-run                # Preview fixes without applying
//! scp recover rollback <ws> <commit>   # Rollback workspace to specific commit
//! scp recover rollback <ws> <commit> --dry-run  # Preview rollback
//! ```

pub mod actions;
pub mod data;

// Re-export public API
pub use actions::{run_recover, run_rollback};
pub use data::{Issue, RecoverOptions, RecoverOutput, RollbackOptions, RollbackOutput};
