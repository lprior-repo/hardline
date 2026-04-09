//! Stack diff command handler - View diffs across stack branches.
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data** (`data.rs`): Options, output types, error enums
//! - **Calc** (`calc.rs`): Pure functions for branch selection, aggregation
//! - **Actions** (`actions.rs`): I/O operations using git CLI
//!
//! Ported from stax `commands/diff.rs` to hardline's functional architecture.
//! Shows per-branch diffs within a stack using three-dot range syntax
//! (`parent...branch`) for merge-base diffs.

pub mod actions;
pub mod calc;
pub mod data;

pub use actions::run_stack_diff;
pub use calc::{aggregate_result, parse_numstat, select_branches};
pub use data::{
    BranchDiff, DiffError, DiffRange, FileStat, StackDiffOptions, StackDiffResult,
};
