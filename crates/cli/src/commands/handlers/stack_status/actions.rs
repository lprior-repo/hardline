//! Actions layer for stack status - I/O operations.
//!
//! This module contains side-effectful operations that interact with the VCS,
//! remote APIs, and display output.

use std::collections::{HashMap, HashSet};

use crate::commands::handlers::stack_status::calc::BranchInfo;
use crate::commands::handlers::stack_status::data::{
    BranchStatusJson, StackStatusOptions, StatusJson,
};

pub fn run_stack_status(
    _options: StackStatusOptions,
    _trunk: String,
    _current: String,
    _display_branches: Vec<crate::commands::handlers::stack_status::data::DisplayBranch>,
    _branch_info_map: HashMap<String, BranchInfo>,
    _linked_worktrees: HashMap<String, String>,
    _remote_branches: HashSet<String>,
    _ci_states: HashMap<String, String>,
    _has_tracked: bool,
    _needs_restack: Vec<String>,
) {
}

#[cfg(test)]
mod tests {
    #[test]
    fn dummy_test() {
        assert!(true);
    }
}
