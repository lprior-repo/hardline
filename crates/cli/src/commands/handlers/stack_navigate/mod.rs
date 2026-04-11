//! Stack navigate command handler - Navigate between stacked branches.
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data** (`data.rs`): Options, output types, error taxonomy
//! - **Calc** (`calc.rs`): Pure functions for navigation target resolution
//! - **Actions** (`actions.rs`): I/O operations using git CLI
//!
//! Provides `up`, `down`, `top`, `bottom`, and `prev` navigation
//! through stacked branch relationships.

pub mod actions;
pub mod calc;
pub mod data;

pub use actions::run_stack_navigate;
pub use calc::resolve_navigate_target;
pub use data::{
    NavigateDirection, NavigateError, StackNavigateOptions, StackNavigateResult,
};
