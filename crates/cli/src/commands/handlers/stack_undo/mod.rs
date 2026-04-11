//! Stack undo command handler - Undo the last stack operation via backup refs.
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data** (`data.rs`): Options, output types, error taxonomy
//! - **Calc** (`calc.rs`): Pure functions for validation and planning
//! - **Actions** (`actions.rs`): I/O operations via git CLI and receipt persistence
//!
//! Restores all branch refs to their pre-operation OIDs using the backup refs
//! created during transaction snapshot. Marks the receipt as undone.

pub mod actions;
pub mod calc;
pub mod data;

pub use actions::{run_stack_redo, run_stack_undo};
pub use calc::{compute_undo_plan, validate_undo_preconditions};
pub use data::{StackUndoOptions, StackUndoOutput, UndoError};
