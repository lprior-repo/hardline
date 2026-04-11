//! Stack split command handler - Split a branch into two at a given commit.
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data** (`data.rs`): Options, output types, error taxonomy
//! - **Calc** (`calc.rs`): Pure functions for validation and planning
//! - **Actions** (`actions.rs`): I/O operations via metadata store and git CLI
//!
//! Ported from stax `commands/split.rs`. The split operation:
//! 1. Validates the branch is tracked and not the trunk
//! 2. Creates lower branch at the split commit
//! 3. Creates upper branch at the original tip
//! 4. Writes metadata for both new branches
//! 5. Reparents children of the source to the upper branch
//! 6. Deletes the source branch's metadata
//! 7. Records a transaction receipt

pub mod actions;
pub mod calc;
pub mod data;

pub use actions::run_stack_split;
pub use calc::{plan_split, resolve_branch_names, validate_split_preconditions};
pub use data::{SplitError, SplitPlan, StackSplitOptions, StackSplitResult};
