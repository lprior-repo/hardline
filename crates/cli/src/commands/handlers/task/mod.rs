//! Task command handler - Manage tasks and work items (beads).
//!
//! Provides subcommands for listing, showing, claiming, yielding, starting,
//! and completing tasks. Tasks are represented as beads in the beads database.
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data** (`data.rs`): TaskCommand, TaskOutput, TaskStatus, TaskListOutput, etc. (inert,
//!   serializable)
//! - **Calculations** (`calculations.rs`): validate_task_command, filter_tasks_by_status,
//!   status_display_icon, parse_task_id (pure functions, no I/O)
//! - **Actions** (`actions.rs`): execute_task_command, run_task_command (I/O operations)
//!
//! # Subcommands
//!
//! - `list` - List all tasks
//! - `show` - Show task details
//! - `claim` - Claim a task for work (uses `LockManager`)
//! - `yield` - Release a claimed task
//! - `start` - Start work on a task (creates session)
//! - `done` - Complete a task

pub mod actions;
pub mod calculations;
pub mod data;

// Re-export public API at module level for convenience
pub use actions::run_task_command;
pub use calculations::parse_task_id;
pub use data::{AgentId, TaskCommand};

#[cfg(test)]
mod tests;
