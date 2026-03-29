//! JJ operation graph synchronization for workspace creation
//!
//! This module solves the problem where multiple concurrent workspace
//! creations can cause operation graph corruption. The issue occurs when:
//!
//! 1. Workspace A is created based on operation X
//! 2. Workspace B is created based on operation Y (sibling of X)
//! 3. Each workspace has its own working copy operation ID
//! 4. JJ detects a mismatch and refuses to load the repo
//!
//! The solution is to ensure all workspace creations are serialized
//! and based on the same repository operation.

mod jj_lock;
mod jj_lock_tests;
mod jj_operations;
mod jj_path;
mod jj_workspace;

pub use jj_lock::{acquire_cross_process_lock, ensure_data_directory, WORKSPACE_CREATION_LOCK};
pub use jj_operations::{create_workspace_synced, get_current_operation, RepoOperationInfo};
pub use jj_path::{get_jj_command, get_jj_command_sync};
