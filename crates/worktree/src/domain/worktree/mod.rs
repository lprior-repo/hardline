//! Worktree type definition

pub mod constructors;
pub mod metadata;
pub mod state_transitions;

use std::collections::HashMap;
use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

use super::{AbsolutePath, BranchName, WorktreeId, WorktreeName, WorktreeState, WorktreeTypeEnum};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Creating;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Incomplete;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Active;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Suspended;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Removing;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Removed;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Worktree<S = Creating> {
    id: WorktreeId,
    name: WorktreeName,
    path: AbsolutePath,
    worktree_type: WorktreeTypeEnum,
    branch: Option<BranchName>,
    parent_path: AbsolutePath,
    created_at: i64,
    updated_at: i64,
    metadata: HashMap<String, String>,
    _state: PhantomData<S>,
}

impl<S> Worktree<S> {
    pub fn id(&self) -> &WorktreeId {
        &self.id
    }

    pub fn name(&self) -> &WorktreeName {
        &self.name
    }

    pub fn name_mut(&mut self) -> &mut WorktreeName {
        &mut self.name
    }

    pub fn path(&self) -> &AbsolutePath {
        &self.path
    }

    pub fn state(&self) -> WorktreeState {
        Self::state_from_marker::<S>()
    }

    fn state_from_marker<S2>() -> WorktreeState {
        if std::any::type_name::<S2>() == std::any::type_name::<Creating>() {
            WorktreeState::Creating
        } else if std::any::type_name::<S2>() == std::any::type_name::<Incomplete>() {
            WorktreeState::Incomplete
        } else if std::any::type_name::<S2>() == std::any::type_name::<Active>() {
            WorktreeState::Active
        } else if std::any::type_name::<S2>() == std::any::type_name::<Suspended>() {
            WorktreeState::Suspended
        } else if std::any::type_name::<S2>() == std::any::type_name::<Removing>() {
            WorktreeState::Removing
        } else if std::any::type_name::<S2>() == std::any::type_name::<Removed>() {
            WorktreeState::Removed
        } else {
            WorktreeState::Creating
        }
    }

    pub fn worktree_type(&self) -> WorktreeTypeEnum {
        self.worktree_type
    }

    pub fn branch(&self) -> Option<&BranchName> {
        self.branch.as_ref()
    }

    pub fn parent_path(&self) -> &AbsolutePath {
        &self.parent_path
    }

    pub fn created_at(&self) -> i64 {
        self.created_at
    }

    pub fn updated_at(&self) -> i64 {
        self.updated_at
    }

    pub fn all_metadata(&self) -> &HashMap<String, String> {
        &self.metadata
    }
}

impl Worktree<Creating> {
    pub fn activate(self) -> Worktree<Active> {
        self.transition_impl()
    }

    pub fn remove(self) -> Worktree<Removed> {
        self.transition_impl()
    }
}

impl Worktree<Incomplete> {
    pub fn activate(self) -> Worktree<Active> {
        self.transition_impl()
    }

    pub fn suspend(self) -> Worktree<Suspended> {
        self.transition_impl()
    }

    pub fn remove(self) -> Worktree<Removed> {
        self.transition_impl()
    }
}

impl Worktree<Active> {
    pub fn suspend(self) -> Worktree<Suspended> {
        self.transition_impl()
    }

    pub fn mark_for_removal(self) -> Worktree<Removing> {
        self.transition_impl()
    }

    pub fn is_active(&self) -> bool {
        true
    }
}

impl Worktree<Suspended> {
    pub fn resume(self) -> Worktree<Active> {
        self.transition_impl()
    }

    pub fn mark_for_removal(self) -> Worktree<Removing> {
        self.transition_impl()
    }
}

impl Worktree<Removing> {
    pub fn complete_removal(self) -> Worktree<Removed> {
        self.transition_impl()
    }
}

impl Worktree<Removed> {
    pub fn is_terminal(&self) -> bool {
        true
    }
}

impl<S> Worktree<S> {
    fn transition_impl<T>(self) -> Worktree<T> {
        Worktree {
            id: self.id,
            name: self.name,
            path: self.path,
            worktree_type: self.worktree_type,
            branch: self.branch,
            parent_path: self.parent_path,
            created_at: self.created_at,
            updated_at: chrono::Utc::now().timestamp(),
            metadata: self.metadata,
            _state: PhantomData,
        }
    }

    /// Convert this worktree to a different state type
    pub fn into_state<T>(self) -> Worktree<T> {
        self.transition_impl()
    }

    /// Check if this worktree is in Removed state
    pub fn is_removed(&self) -> bool {
        self.state() == WorktreeState::Removed
    }
}

/// Conversions from specific state types to the default Worktree (Creating)
impl From<Worktree<Active>> for Worktree {
    fn from(other: Worktree<Active>) -> Self {
        other.into_state()
    }
}

impl From<Worktree<Incomplete>> for Worktree {
    fn from(other: Worktree<Incomplete>) -> Self {
        other.into_state()
    }
}

impl From<Worktree<Suspended>> for Worktree {
    fn from(other: Worktree<Suspended>) -> Self {
        other.into_state()
    }
}

impl From<Worktree<Removing>> for Worktree {
    fn from(other: Worktree<Removing>) -> Self {
        other.into_state()
    }
}

impl From<Worktree<Removed>> for Worktree {
    fn from(other: Worktree<Removed>) -> Self {
        other.into_state()
    }
}

impl Default for WorktreeState {
    fn default() -> Self {
        Self::Creating
    }
}
