//! Stack range-diff command handler - Inter-branch range comparison.
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data** (`data.rs`): Options, result types, error taxonomy
//! - **Calc** (`calc.rs`): Pure functions for range building and output parsing
//! - **Actions** (`actions.rs`): I/O operations via git CLI
//!
//! Compares two commit ranges and shows how patches changed between them.
//! Useful for seeing what changed after a rebase, restack, or other
//! branch transformation.

pub mod actions;
pub mod calc;
pub mod data;

pub use actions::{run_range_diff, run_range_diff_branches};
pub use calc::{
    build_git_args, build_range_args, build_result, compare_branch_ranges, format_flag,
    has_changes, parse_range_diff_output, validate_refs,
};
pub use data::{
    CommitPairing, CommitSummary, PairingStatus, RangeDiffError, RangeDiffFormat,
    RangeDiffOptions, RangeDiffResult, RangeSpec,
};
