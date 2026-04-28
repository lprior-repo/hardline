//! Conflict state management for branch operations
//!
//! Provides types for tracking and resolving merge conflicts.

use serde::{Deserialize, Serialize};

/// State of a conflict in a branch
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ConflictState {
    /// No conflict present
    #[default]
    None,
    /// Conflict detected, needs resolution
    Detected,
    /// Conflict is being resolved
    Resolving,
    /// Conflict resolved successfully
    Resolved,
    /// Conflict resolution failed
    Failed,
}

impl std::fmt::Display for ConflictState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::Detected => write!(f, "detected"),
            Self::Resolving => write!(f, "resolving"),
            Self::Resolved => write!(f, "resolved"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

impl ConflictState {
    /// Check if this is a terminal state
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Resolved | Self::Failed | Self::None)
    }

    /// Check if conflict needs resolution
    #[must_use]
    pub const fn needs_resolution(self) -> bool {
        matches!(self, Self::Detected | Self::Resolving)
    }

    /// Try to transition to a new state
    ///
    /// # Errors
    /// Returns `ConflictError::InvalidStateTransition` if the transition is not allowed.
    pub fn transition_to(self, new_state: Self) -> Result<Self, crate::Error> {
        let is_valid = matches!(
            (self, new_state),
            // Valid transitions
            (Self::None | Self::Resolved | Self::Failed, Self::Detected)
                | (Self::Detected, Self::Resolving | Self::None)
                | (Self::Resolving, Self::Resolved | Self::Failed)
                | (Self::Failed, Self::None)
        );

        if is_valid {
            Ok(new_state)
        } else {
            Err(crate::Error::InvalidState(format!(
                "Invalid conflict state transition from {} to {}",
                self, new_state
            )))
        }
    }
}

/// Conflict information for a branch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    /// Branch identifier with conflict
    pub branch_id: String,
    /// Current state of the conflict
    pub state: ConflictState,
    /// Description of the conflict
    pub description: String,
    /// Base commit SHA for conflict
    pub base_commit: Option<String>,
    /// Conflicting commit SHAs
    pub conflicting_commits: Vec<String>,
    /// When the conflict was detected
    pub detected_at: Option<chrono::DateTime<chrono::Utc>>,
    /// When the conflict was resolved (if applicable)
    pub resolved_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl Conflict {
    /// Create a new conflict with detected state
    #[must_use]
    pub fn new(branch_id: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            branch_id: branch_id.into(),
            state: ConflictState::Detected,
            description: description.into(),
            base_commit: None,
            conflicting_commits: Vec::new(),
            detected_at: Some(chrono::Utc::now()),
            resolved_at: None,
        }
    }

    /// Mark conflict as resolving
    ///
    /// # Errors
    /// Returns an error if the state transition is invalid.
    pub fn start_resolution(&mut self) -> Result<(), crate::Error> {
        self.state = self.state.transition_to(ConflictState::Resolving)?;
        Ok(())
    }

    /// Mark conflict as resolved
    ///
    /// # Errors
    /// Returns an error if the state transition is invalid.
    pub fn resolve(&mut self) -> Result<(), crate::Error> {
        self.state = self.state.transition_to(ConflictState::Resolved)?;
        self.resolved_at = Some(chrono::Utc::now());
        Ok(())
    }

    /// Mark conflict as failed
    ///
    /// # Errors
    /// Returns an error if the state transition is invalid.
    pub fn fail(&mut self) -> Result<(), crate::Error> {
        self.state = self.state.transition_to(ConflictState::Failed)?;
        Ok(())
    }

    /// Check if conflict is resolved
    #[must_use]
    pub fn is_resolved(&self) -> bool {
        self.state == ConflictState::Resolved
    }

    /// Check if conflict needs resolution
    #[must_use]
    pub fn needs_resolution(&self) -> bool {
        self.state.needs_resolution()
    }
}

/// Conflict manager for tracking and resolving conflicts
#[derive(Debug, Default)]
pub struct ConflictManager {
    conflicts: std::collections::HashMap<String, Conflict>,
}

impl ConflictManager {
    /// Create a new conflict manager
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new conflict
    ///
    /// # Errors
    /// Returns an error if the branch already has an unresolved conflict.
    pub fn register_conflict(&mut self, conflict: Conflict) -> Result<(), crate::Error> {
        let branch_id = conflict.branch_id.clone();

        if let Some(existing) = self.conflicts.get(&branch_id) {
            if existing.needs_resolution() {
                return Err(crate::Error::InvalidState(format!(
                    "Branch '{}' has unresolved conflicts",
                    branch_id
                )));
            }
        }

        self.conflicts.insert(branch_id, conflict);
        Ok(())
    }

    /// Get conflict for a branch
    #[must_use]
    pub fn get_conflict(&self, branch_id: &str) -> Option<&Conflict> {
        self.conflicts.get(branch_id)
    }

    /// Get mutable conflict for a branch
    pub fn get_conflict_mut(&mut self, branch_id: &str) -> Option<&mut Conflict> {
        self.conflicts.get_mut(branch_id)
    }

    /// Start resolving a conflict
    ///
    /// # Errors
    /// Returns an error if no conflict exists for the branch.
    pub fn start_resolution(&mut self, branch_id: &str) -> Result<(), crate::Error> {
        let conflict = self.conflicts.get_mut(branch_id).ok_or_else(|| {
            crate::Error::NotFound(format!("No conflict for branch: {}", branch_id))
        })?;

        conflict.start_resolution()
    }

    /// Mark a conflict as resolved
    ///
    /// # Errors
    /// Returns an error if no conflict exists for the branch.
    pub fn resolve(&mut self, branch_id: &str) -> Result<(), crate::Error> {
        let conflict = self.conflicts.get_mut(branch_id).ok_or_else(|| {
            crate::Error::NotFound(format!("No conflict for branch: {}", branch_id))
        })?;

        conflict.resolve()
    }

    /// Mark a conflict as failed
    ///
    /// # Errors
    /// Returns an error if no conflict exists for the branch.
    pub fn fail(&mut self, branch_id: &str) -> Result<(), crate::Error> {
        let conflict = self.conflicts.get_mut(branch_id).ok_or_else(|| {
            crate::Error::NotFound(format!("No conflict for branch: {}", branch_id))
        })?;

        conflict.fail()
    }

    /// Remove a conflict from tracking
    pub fn remove(&mut self, branch_id: &str) -> Option<Conflict> {
        self.conflicts.remove(branch_id)
    }

    /// Get all conflicts that need resolution
    #[must_use]
    pub fn unresolved_conflicts(&self) -> Vec<&Conflict> {
        self.conflicts
            .values()
            .filter(|c| c.needs_resolution())
            .collect()
    }

    /// Check if a branch has unresolved conflicts
    #[must_use]
    pub fn has_conflict(&self, branch_id: &str) -> bool {
        self.conflicts
            .get(branch_id)
            .is_some_and(Conflict::needs_resolution)
    }

    /// Clear all conflicts
    pub fn clear(&mut self) {
        self.conflicts.clear();
    }

    /// Get number of tracked conflicts
    #[must_use]
    pub fn len(&self) -> usize {
        self.conflicts.len()
    }

    /// Check if there are no conflicts tracked
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.conflicts.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conflict_state_transitions() {
        let state = ConflictState::None;
        let new_state = state.transition_to(ConflictState::Detected).unwrap();
        assert_eq!(new_state, ConflictState::Detected);
    }

    #[test]
    fn test_conflict_state_invalid_transition() {
        let state = ConflictState::Resolved;
        let result = state.transition_to(ConflictState::Resolving);
        assert!(result.is_err());
    }

    #[test]
    fn test_conflict_manager_register() {
        let mut manager = ConflictManager::new();
        let conflict = Conflict::new("feature", "Merge conflict detected");
        manager.register_conflict(conflict).unwrap();
        assert!(manager.has_conflict("feature"));
    }

    #[test]
    fn test_conflict_resolution_flow() {
        let mut manager = ConflictManager::new();
        let conflict = Conflict::new("feature", "Merge conflict detected");
        manager.register_conflict(conflict).unwrap();

        manager.start_resolution("feature").unwrap();
        manager.resolve("feature").unwrap();

        let conflict = manager.get_conflict("feature").unwrap();
        assert!(conflict.is_resolved());
    }

    #[test]
    fn test_unresolved_conflicts() {
        let mut manager = ConflictManager::new();
        let conflict1 = Conflict::new("feature1", "Conflict 1");
        let conflict2 = Conflict::new("feature2", "Conflict 2");
        manager.register_conflict(conflict1).unwrap();
        manager.register_conflict(conflict2).unwrap();

        let unresolved = manager.unresolved_conflicts();
        assert_eq!(unresolved.len(), 2);
    }
}
