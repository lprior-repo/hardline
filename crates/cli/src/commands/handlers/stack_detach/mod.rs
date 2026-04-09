//! Stack detach command handler - Detach a branch from the stack, reparent children.
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data** (`data.rs`): Options, output types, error taxonomy
//! - **Calc** (`calc.rs`): Pure functions for validation and planning
//! - **Actions** (`actions.rs`): I/O operations via metadata store and git CLI
//!
//! Ported from stax `commands/detach.rs`. The detach operation:
//! 1. Validates the branch is tracked and not the trunk
//! 2. Plans which children need reparenting
//! 3. Reparents children to the detached branch's parent
//! 4. Deletes the detached branch's metadata
//! 5. Optionally deletes the local git branch
//! 6. Records a transaction receipt

pub mod actions;
pub mod calc;
pub mod data;

pub use actions::run_stack_detach;
pub use calc::{plan_detach, validate_detach_preconditions};
pub use data::{DetachError, StackDetachOptions, StackDetachResult};
