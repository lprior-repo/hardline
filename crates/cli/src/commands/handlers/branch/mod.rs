//! Branch command handler - Create, delete, and rename branches.
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data** (`data.rs`): Options, Output types, validation functions
//! - **Actions** (`actions.rs`): run_branch_create, run_branch_delete, run_branch_rename
//!
//! # CLI Usage
//!
//! ```text
//! scp workspace branch <name>                          # Create branch
//! scp workspace branch-delete <name> [--force]   # Delete branch
//! scp workspace branch-rename <old> <new> [--dry-run]  # Rename branch
//! ```

pub mod actions;
pub mod data;

pub use actions::{run_branch_create, run_branch_delete, run_branch_rename};
pub use data::{
    is_protected_branch, validate_branch_name, BranchCreateOptions, BranchCreateOutput, BranchDeleteOptions,
    BranchRenameOptions,
};
