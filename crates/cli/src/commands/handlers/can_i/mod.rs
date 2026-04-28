//! Can-I command handler - Check if an action is permitted.
//!
//! Allows AI agents and users to check preconditions before attempting
//! operations. Reports whether an action is allowed, the reason, any
//! prerequisite checks, and suggested fix commands.
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data** (`data.rs`): CanIOptions, CanIOutput, Prerequisite, helper functions (inert,
//!   serializable types)
//! - **Actions** (`actions.rs`): run_can_i, check_permission, per-action checks (I/O operations)
//!
//! # CLI Usage
//!
//! ```text
//! scp can-i spawn bead-123       # Check if spawn is allowed
//! scp can-i done my-workspace    # Check if done is allowed
//! scp can-i undo                 # Check if undo is allowed
//! scp can-i custom-action        # Unknown actions are generally allowed
//! ```

pub mod actions;
pub mod data;

pub use actions::run_can_i;
pub use data::{CanIOptions, CanIOutput, Prerequisite};
