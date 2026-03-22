//! JJ workspace lifecycle management
//!
//! This module provides safe, functional APIs for managing JJ workspaces.
//! All operations return `Result` and never panic.

pub mod command;
pub mod conflict;
pub mod parse;
pub mod types;
pub mod workspace_guard;
pub mod workspace_ops;

// Re-exports for convenience
pub use command::{get_jj_command, get_jj_command_sync};
pub use conflict::{conflict_recovery_hint, detect_workspace_conflict};
pub use parse::{parse_diff_stat, parse_status, parse_workspace_list};
pub use types::{DiffSummary, Status, WorkspaceInfo};
pub use workspace_guard::WorkspaceGuard;
pub use workspace_ops::{
    check_in_jj_repo, check_jj_installed, create_workspace, is_jj_installed, is_jj_repo,
    workspace_create, workspace_diff, workspace_forget, workspace_list, workspace_status,
};
