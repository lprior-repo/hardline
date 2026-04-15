//! Stack status command handler - Display stack tree visualization with CI status.
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data** (`data.rs`): Options, output types, display constants
//! - **Calc** (`calc.rs`): Pure functions for tree layout, branch status computation
//! - **Actions** (`actions.rs`): I/O operations, VCS queries, display output
//!
//! Ported from stax `commands/status.rs` (733 lines) to hardline's functional architecture.

pub mod actions;
pub mod calc;
pub mod data;

pub use calc::{
    build_branch_statuses, compute_display_branches, compute_stats, format_compact_line,
    format_tree_element, format_trunk_display, BranchInfo,
};
pub use data::{
    BranchStatusJson, DisplayBranch, StackStatusOptions, StatusJson, COLUMN_COLORS,
    LINKED_WORKTREE_GLYPH,
};
