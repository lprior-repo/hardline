//! Workspace navigation calculation functions (pure)
//!
//! These functions have no side effects and are fully testable.

use crate::commands::workspace::operations::{
    find_next_workspace as ops_find_next, find_prev_workspace as ops_find_prev,
    sorted_workspace_names as ops_sorted,
};
use crate::commands::workspace::validators::validate_workspace_name;
use scp_core::vcs::Workspace;

pub use crate::commands::workspace::validators::validate_workspace_name as validate_name;

pub fn sorted_workspace_names(workspaces: &[Workspace]) -> Vec<String> {
    ops_sorted(workspaces)
}

pub fn find_next_workspace(workspaces: &[Workspace]) -> Result<String, scp_core::Error> {
    ops_find_next(workspaces)
}

pub fn find_prev_workspace(workspaces: &[Workspace]) -> Result<String, scp_core::Error> {
    ops_find_prev(workspaces)
}

pub fn validate_spawn_name(name: &str) -> Option<scp_core::Error> {
    validate_workspace_name(name)
}

pub fn validate_switch_name(name: &str) -> Option<scp_core::Error> {
    validate_workspace_name(name)
}
