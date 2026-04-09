//! Stack sync command handler - Fetch, detect drift, auto-restack.
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data** (`data.rs`): Options, output types, detection enums
//! - **Calc** (`calc.rs`): Pure functions for merged detection, restack planning
//! - **Actions** (`actions.rs`): I/O operations using VcsBackend
//!
//! Ported from stax `commands/sync.rs` (1539 lines) to hardline's functional architecture.

pub mod actions;
pub mod calc;
pub mod data;

pub use actions::run_stack_sync;
pub use calc::{
    compute_drift, detect_merged_branches, plan_restack_order, resolve_effective_parent,
};
pub use data::{
    DriftReport, MergedBranch, MergedDetectionMethod, RestackOutcome, RestackStatus,
    StackSyncOptions, StackSyncResult, SyncError,
};
