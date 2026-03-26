//! Worktree constructors and factories

use std::collections::HashMap;
use std::marker::PhantomData;

use chrono::Utc;

use super::{Worktree, WorktreeState};
use crate::domain::{AbsolutePath, BranchName, WorktreeId, WorktreeName, WorktreeTypeEnum};

impl Worktree<super::Creating> {
    pub fn new(
        name: WorktreeName,
        path: AbsolutePath,
        parent_path: AbsolutePath,
        worktree_type: WorktreeTypeEnum,
        branch: Option<BranchName>,
    ) -> Self {
        let now = Utc::now().timestamp();

        Self {
            id: WorktreeId::new_random(),
            name,
            path,
            worktree_type,
            branch,
            parent_path,
            created_at: now,
            updated_at: now,
            metadata: HashMap::new(),
            _state: PhantomData,
        }
    }
}

impl<S> Worktree<S> {
    #[allow(clippy::too_many_arguments)]
    pub fn uninitialized(
        id: WorktreeId,
        name: WorktreeName,
        path: AbsolutePath,
        parent_path: AbsolutePath,
        worktree_type: WorktreeTypeEnum,
        branch: Option<BranchName>,
        state: WorktreeState,
        created_at: i64,
        updated_at: i64,
    ) -> Worktree<S> {
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

    #[allow(clippy::too_many_arguments)]
    pub fn uninitialized_with_metadata(
        id: WorktreeId,
        name: WorktreeName,
        path: AbsolutePath,
        parent_path: AbsolutePath,
        worktree_type: WorktreeTypeEnum,
        branch: Option<BranchName>,
        _state: WorktreeState,
        created_at: i64,
        updated_at: i64,
        metadata: HashMap<String, String>,
    ) -> Worktree<S> {
        Worktree {
            id,
            name,
            path,
            worktree_type,
            branch,
            parent_path,
            created_at,
            updated_at,
            metadata,
            _state: PhantomData,
        }
    }
}
