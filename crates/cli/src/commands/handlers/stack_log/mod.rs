//! Stack log command handler - Stack-aware commit log.
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data** (`data.rs`): Options, output types, error taxonomy
//! - **Calc** (`calc.rs`): Pure functions for depth computation, lineage filtering, formatting
//! - **Actions** (`actions.rs`): I/O operations using git CLI
//!
//! Ported from stax `commands/log.rs` to hardline's functional architecture.

pub mod actions;
pub mod calc;
pub mod data;

pub use actions::{load_stack_from_git, run_stack_log};
pub use calc::{collect_needs_restack, compute_depths, count_total_commits, filter_to_lineage, format_linear, format_tree};
pub use data::{LogError, StackLogBranchEntry, StackLogCommit, StackLogOptions, StackLogOutput};
