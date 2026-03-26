//! Worktree constructors and factories

use chrono::Utc;
use std::collections::HashMap;

use super::{Worktree, WorktreeId, WorktreeName, WorktreeState, WorktreeTypeEnum};
use crate::domain::WorktreeDomainError;

impl Worktree {
    /// Create a new worktree with initial validation
    pub fn new(
        name: WorktreeName,
        path: super::super::AbsolutePath,
        parent_path: super::super::AbsolutePath,
        worktree_type: WorktreeTypeEnum,
        branch: Option<super::super::BranchName>,
    ) -> Result<Self, WorktreeDomainError> {
        let now = Utc::now().timestamp();

        Ok(Self {
            id: WorktreeId::new_random(),
            name,
            path,
            state: WorktreeState::Creating,
            worktree_type,
            branch,
            parent_path,
            created_at: now,
            updated_at: now,
            metadata: HashMap::new(),
        })
    }

    /// Create an uninitialized worktree (for database loading)
    pub fn uninitialized(
        id: WorktreeId,
        name: WorktreeName,
        path: super::super::AbsolutePath,
        parent_path: super::super::AbsolutePath,
        worktree_type: WorktreeTypeEnum,
        branch: Option<super::super::BranchName>,
        state: WorktreeState,
        created_at: i64,
        updated_at: i64,
    ) -> Self {
        Self::uninitialized_with_metadata(
            id,
            name,
            path,
            parent_path,
            worktree_type,
            branch,
            state,
            created_at,
            updated_at,
            HashMap::new(),
        )
    }

    /// Create an uninitialized worktree with metadata (for database loading)
    pub fn uninitialized_with_metadata(
        id: WorktreeId,
        name: WorktreeName,
        path: super::super::AbsolutePath,
        parent_path: super::super::AbsolutePath,
        worktree_type: WorktreeTypeEnum,
        branch: Option<super::super::BranchName>,
        state: WorktreeState,
        created_at: i64,
        updated_at: i64,
        metadata: HashMap<String, String>,
    ) -> Self {
        Self {
            id,
            name,
            path,
            state,
            worktree_type,
            branch,
            parent_path,
            created_at,
            updated_at,
            metadata,
        }
    }
}
