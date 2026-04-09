//! Stack merge-remote command handler — merge PRs via GitHub API, no local checkout.
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data** (`data.rs`): Options, output types, error taxonomy
//! - **Calc** (`calc.rs`): Pure functions for scope calculation, PR resolution, wait logic
//! - **Actions** (`actions.rs`): Forge API calls via GitHubClient
//!
//! Ported from stax `commands/merge_remote.rs` (690 lines) to hardline's functional architecture.

pub mod actions;
pub mod calc;
pub mod data;

pub use actions::run_merge_remote;
pub use calc::{calculate_merge_scope, resolve_pr_numbers};
pub use data::{
    MergeRemoteOptions, MergeRemoteOutput, MergeRemoteScope, MergedPr, PrBranchInfo,
    WaitOutcome,
};
