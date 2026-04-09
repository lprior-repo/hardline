//! Workspace navigation command handler.
//!
//! Provides subcommands for workspace navigation operations:
//! - `spawn` - Create a new workspace
//! - `switch` - Switch to a workspace
//! - `list` - List all workspaces
//! - `status` - Show workspace status
//! - `next` - Switch to next workspace alphabetically
//! - `prev` - Switch to previous workspace alphabetically
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data** (`data.rs`): WorkspaceNavCommand, WorkspaceNavOutput types
//! - **Calculations** (`calculations.rs`): validate_workspace_name, sorted_workspace_names,
//!   find_next_workspace, find_prev_workspace (pure functions)
//! - **Actions** (`actions.rs`): run_workspace_nav_command (I/O operations)
//!
//! # Error Handling
//!
//! All functions use Result<T, Error> for railway-oriented error propagation.
//! Common errors:
//! - `Error::InvalidIdentifier` - Invalid workspace name
//! - `Error::WorkspaceNotFound` - Workspace does not exist
//! - `Error::WorkspaceExists` - Workspace already exists
//! - `Error::WorkingCopyDirty` - Working copy has uncommitted changes

pub mod calculations;
pub mod data;

#[cfg(test)]
pub(crate) mod tests;
