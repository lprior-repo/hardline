//! Worktree type definition

pub mod constructors;
pub mod metadata;
pub mod state_transitions;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{AbsolutePath, BranchName, WorktreeId, WorktreeName, WorktreeState, WorktreeTypeEnum};

/// Aggregate root representing a Git worktree
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Worktree {
    /// Unique identifier for this worktree
    id: WorktreeId,

    /// Human-readable name for this worktree
    name: WorktreeName,

    /// Absolute path to the worktree location
    path: AbsolutePath,

    /// Current state of the worktree
    state: WorktreeState,

    /// Type of worktree (development, testing, etc.)
    worktree_type: WorktreeTypeEnum,

    /// Branch associated with this worktree
    branch: Option<BranchName>,

    /// Path to the parent repository
    parent_path: AbsolutePath,

    /// Creation timestamp (Unix epoch seconds)
    created_at: i64,

    /// Last modification timestamp (Unix epoch seconds)
    updated_at: i64,

    /// Custom metadata key-value pairs
    metadata: HashMap<String, String>,
}

impl Worktree {
    /// Get the unique identifier
    pub fn id(&self) -> &WorktreeId {
        &self.id
    }

    /// Get the worktree name
    pub fn name(&self) -> &WorktreeName {
        &self.name
    }

    /// Get mutable access to the name
    pub fn name_mut(&mut self) -> &mut WorktreeName {
        &mut self.name
    }

    /// Get the worktree path
    pub fn path(&self) -> &AbsolutePath {
        &self.path
    }

    /// Get the current state
    pub fn state(&self) -> WorktreeState {
        self.state
    }

    /// Get the worktree type
    pub fn worktree_type(&self) -> WorktreeTypeEnum {
        self.worktree_type
    }

    /// Get the associated branch if any
    pub fn branch(&self) -> Option<&BranchName> {
        self.branch.as_ref()
    }

    /// Get the parent repository path
    pub fn parent_path(&self) -> &AbsolutePath {
        &self.parent_path
    }

    /// Get creation timestamp
    pub fn created_at(&self) -> i64 {
        self.created_at
    }

    /// Get last update timestamp
    pub fn updated_at(&self) -> i64 {
        self.updated_at
    }

    /// Check if worktree is in active state
    pub fn is_active(&self) -> bool {
        self.state.is_active()
    }

    /// Check if worktree is in a terminal state (removed)
    pub fn is_removed(&self) -> bool {
        self.state.is_terminal()
    }

    /// Get all metadata
    pub fn all_metadata(&self) -> &HashMap<String, String> {
        &self.metadata
    }
}
