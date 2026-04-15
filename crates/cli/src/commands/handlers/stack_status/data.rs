//! Data layer for stack status - inert, serializable types.
//!
//! No business logic. Types only.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use scp_stack::BranchName;

#[derive(Debug, Clone)]
pub struct StackStatusOptions {
    pub json: bool,
    pub stack_filter: Option<String>,
    pub current_only: bool,
    pub compact: bool,
    pub quiet: bool,
    pub verbose: bool,
}

impl Default for StackStatusOptions {
    fn default() -> Self {
        Self {
            json: false,
            stack_filter: None,
            current_only: false,
            compact: false,
            quiet: false,
            verbose: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchStatusJson {
    pub name: String,
    pub parent: Option<String>,
    pub is_current: bool,
    pub is_trunk: bool,
    pub linked_worktree: Option<String>,
    pub needs_restack: bool,
    pub pr_number: Option<u64>,
    pub pr_state: Option<String>,
    pub pr_is_draft: Option<bool>,
    pub pr_url: Option<String>,
    pub ci_state: Option<String>,
    pub ahead: usize,
    pub behind: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lines_added: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lines_deleted: Option<usize>,
    pub has_remote: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusJson {
    pub trunk: String,
    pub current: String,
    pub branches: Vec<BranchStatusJson>,
}

#[derive(Debug, Clone)]
pub struct DisplayBranch {
    pub name: String,
    pub column: usize,
}

pub const COLUMN_COLORS: &[(&str, &str)] = &[
    ("cyan", "\u{1b}[36m"),
    ("green", "\u{1b}[32m"),
    ("magenta", "\u{1b}[35m"),
    ("blue", "\u{1b}[34m"),
    ("bright_cyan", "\u{1b}[96m"),
    ("bright_green", "\u{1b}[92m"),
    ("bright_magenta", "\u{1b}[95m"),
    ("bright_blue", "\u{1b}[94m"),
];

pub const LINKED_WORKTREE_GLYPH: &str = "\u{21b3}";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stack_status_options_default() {
        let opts = StackStatusOptions::default();
        assert!(!opts.json);
        assert!(!opts.current_only);
        assert!(!opts.compact);
        assert!(!opts.quiet);
        assert!(!opts.verbose);
        assert!(opts.stack_filter.is_none());
    }

    #[test]
    fn branch_status_json_serde() {
        let status = BranchStatusJson {
            name: "feature-a".to_string(),
            parent: Some("main".to_string()),
            is_current: false,
            is_trunk: false,
            linked_worktree: None,
            needs_restack: false,
            pr_number: Some(42),
            pr_state: Some("open".to_string()),
            pr_is_draft: Some(false),
            pr_url: Some("https://github.com/org/repo/pull/42".to_string()),
            ci_state: Some("success".to_string()),
            ahead: 3,
            behind: 0,
            lines_added: Some(50),
            lines_deleted: Some(10),
            has_remote: true,
        };
        let json = serde_json::to_string(&status).expect("serialize");
        let deserialized: BranchStatusJson = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(status.name, deserialized.name);
        assert_eq!(status.pr_number, deserialized.pr_number);
    }

    #[test]
    fn status_json_serde() {
        let status = StatusJson {
            trunk: "main".to_string(),
            current: "feature-a".to_string(),
            branches: vec![BranchStatusJson {
                name: "feature-a".to_string(),
                parent: Some("main".to_string()),
                is_current: true,
                is_trunk: false,
                linked_worktree: None,
                needs_restack: false,
                pr_number: None,
                pr_state: None,
                pr_is_draft: None,
                pr_url: None,
                ci_state: None,
                ahead: 0,
                behind: 0,
                lines_added: None,
                lines_deleted: None,
                has_remote: false,
            }],
        };
        let json = serde_json::to_string(&status).expect("serialize");
        let deserialized: StatusJson = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(status.trunk, deserialized.trunk);
        assert_eq!(status.branches.len(), 1);
    }

    #[test]
    fn display_branch_creation() {
        let db = DisplayBranch {
            name: "feature-a".to_string(),
            column: 0,
        };
        assert_eq!(db.name, "feature-a");
        assert_eq!(db.column, 0);
    }

    #[test]
    fn column_colors_length() {
        assert_eq!(COLUMN_COLORS.len(), 8);
    }
}
