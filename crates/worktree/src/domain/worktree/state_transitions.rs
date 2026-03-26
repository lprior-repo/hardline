//! State machine transitions for worktree lifecycle

use chrono::Utc;

use super::{Worktree, WorktreeState};
use crate::domain::WorktreeDomainError;

impl Worktree {
    /// Initialize the worktree (transition to Active state)
    pub fn initialize(&mut self) -> Result<(), WorktreeDomainError> {
        if !self.state.can_transition_to(WorktreeState::Active) {
            return Err(WorktreeDomainError::InvalidStateTransition(
                self.state,
                WorktreeState::Active,
            ));
        }

        self.state = WorktreeState::Active;
        self.updated_at = Utc::now().timestamp();
        Ok(())
    }

    /// Suspend the worktree
    pub fn suspend(&mut self) -> Result<(), WorktreeDomainError> {
        if !self.state.can_transition_to(WorktreeState::Suspended) {
            return Err(WorktreeDomainError::InvalidStateTransition(
                self.state,
                WorktreeState::Suspended,
            ));
        }

        self.state = WorktreeState::Suspended;
        self.updated_at = Utc::now().timestamp();
        Ok(())
    }

    /// Resume a suspended worktree
    pub fn resume(&mut self) -> Result<(), WorktreeDomainError> {
        if !self.state.can_transition_to(WorktreeState::Active) {
            return Err(WorktreeDomainError::InvalidStateTransition(
                self.state,
                WorktreeState::Active,
            ));
        }

        self.state = WorktreeState::Active;
        self.updated_at = Utc::now().timestamp();
        Ok(())
    }

    /// Mark worktree for removal
    pub fn mark_for_removal(&mut self) -> Result<(), WorktreeDomainError> {
        if !self.state.can_transition_to(WorktreeState::Removing) {
            return Err(WorktreeDomainError::InvalidStateTransition(
                self.state,
                WorktreeState::Removing,
            ));
        }

        self.state = WorktreeState::Removing;
        self.updated_at = Utc::now().timestamp();
        Ok(())
    }

    /// Complete removal of worktree
    pub fn complete_removal(&mut self) -> Result<(), WorktreeDomainError> {
        if !self.state.can_transition_to(WorktreeState::Removed) {
            return Err(WorktreeDomainError::InvalidStateTransition(
                self.state,
                WorktreeState::Removed,
            ));
        }

        self.state = WorktreeState::Removed;
        self.updated_at = Utc::now().timestamp();
        Ok(())
    }
}
