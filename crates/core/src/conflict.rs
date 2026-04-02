#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

//! Conflict state management for branch operations
//!
//! Provides types for tracking and resolving merge conflicts.

use serde::{Deserialize, Serialize};

use crate::error::Error;

pub type Result<T> = std::result::Result<T, Error>;

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
    /// Returns `Error::InvalidState` if the transition is not allowed.
    pub fn transition_to(self, new_state: Self) -> Result<Self> {
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
            Err(Error::invalid_state(format!(
                "Invalid conflict state transition from {self} to {new_state}"
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
    pub fn start_resolution(&mut self) -> Result<()> {
        self.state = self.state.transition_to(ConflictState::Resolving)?;
        Ok(())
    }

    /// Mark conflict as resolved
    ///
    /// # Errors
    /// Returns an error if the state transition is invalid.
    pub fn resolve(&mut self) -> Result<()> {
        self.state = self.state.transition_to(ConflictState::Resolved)?;
        self.resolved_at = Some(chrono::Utc::now());
        Ok(())
    }

    /// Mark conflict as failed
    ///
    /// # Errors
    /// Returns an error if the state transition is invalid.
    pub fn fail(&mut self) -> Result<()> {
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
    pub fn register_conflict(&mut self, conflict: Conflict) -> Result<()> {
        let branch_id = conflict.branch_id.clone();

        if let Some(existing) = self.conflicts.get(&branch_id) {
            if existing.needs_resolution() {
                return Err(Error::invalid_state(format!(
                    "Branch '{branch_id}' has unresolved conflicts"
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
    pub fn start_resolution(&mut self, branch_id: &str) -> Result<()> {
        let conflict = self
            .conflicts
            .get_mut(branch_id)
            .ok_or_else(|| Error::not_found(format!("No conflict for branch: {branch_id}")))?;

        conflict.start_resolution()
    }

    /// Mark a conflict as resolved
    ///
    /// # Errors
    /// Returns an error if no conflict exists for the branch.
    pub fn resolve(&mut self, branch_id: &str) -> Result<()> {
        let conflict = self
            .conflicts
            .get_mut(branch_id)
            .ok_or_else(|| Error::not_found(format!("No conflict for branch: {branch_id}")))?;

        conflict.resolve()
    }

    /// Mark a conflict as failed
    ///
    /// # Errors
    /// Returns an error if no conflict exists for the branch.
    pub fn fail(&mut self, branch_id: &str) -> Result<()> {
        let conflict = self
            .conflicts
            .get_mut(branch_id)
            .ok_or_else(|| Error::not_found(format!("No conflict for branch: {branch_id}")))?;

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

    // ── ConflictState enum ───────────────────────────────────────────────────

    #[test]
    fn test_conflict_state_all_variants() {
        let all = [
            ConflictState::None,
            ConflictState::Detected,
            ConflictState::Resolving,
            ConflictState::Resolved,
            ConflictState::Failed,
        ];
        assert_eq!(all.len(), 5);
    }

    #[test]
    fn test_conflict_state_default() {
        assert_eq!(ConflictState::default(), ConflictState::None);
    }

    #[test]
    fn test_conflict_state_display_all() {
        assert_eq!(ConflictState::None.to_string(), "none");
        assert_eq!(ConflictState::Detected.to_string(), "detected");
        assert_eq!(ConflictState::Resolving.to_string(), "resolving");
        assert_eq!(ConflictState::Resolved.to_string(), "resolved");
        assert_eq!(ConflictState::Failed.to_string(), "failed");
    }

    #[test]
    fn test_conflict_state_is_terminal() {
        assert!(ConflictState::None.is_terminal());
        assert!(!ConflictState::Detected.is_terminal());
        assert!(!ConflictState::Resolving.is_terminal());
        assert!(ConflictState::Resolved.is_terminal());
        assert!(ConflictState::Failed.is_terminal());
    }

    #[test]
    fn test_conflict_state_needs_resolution() {
        assert!(!ConflictState::None.needs_resolution());
        assert!(ConflictState::Detected.needs_resolution());
        assert!(ConflictState::Resolving.needs_resolution());
        assert!(!ConflictState::Resolved.needs_resolution());
        assert!(!ConflictState::Failed.needs_resolution());
    }

    // ── ConflictState transitions ────────────────────────────────────────────

    #[test]
    fn test_conflict_state_valid_transitions() {
        // None -> Detected
        assert!(ConflictState::None.transition_to(ConflictState::Detected).is_ok());
        // Detected -> Resolving
        assert!(ConflictState::Detected.transition_to(ConflictState::Resolving).is_ok());
        // Detected -> None (cancel)
        assert!(ConflictState::Detected.transition_to(ConflictState::None).is_ok());
        // Resolving -> Resolved
        assert!(ConflictState::Resolving.transition_to(ConflictState::Resolved).is_ok());
        // Resolving -> Failed
        assert!(ConflictState::Resolving.transition_to(ConflictState::Failed).is_ok());
        // Resolved -> Detected (re-conflict)
        assert!(ConflictState::Resolved.transition_to(ConflictState::Detected).is_ok());
        // Failed -> None (reset)
        assert!(ConflictState::Failed.transition_to(ConflictState::None).is_ok());
        // Failed -> Detected (retry)
        assert!(ConflictState::Failed.transition_to(ConflictState::Detected).is_ok());
    }

    #[test]
    fn test_conflict_state_invalid_transitions() {
        // Resolved -> Resolving (can't go back to resolving from resolved)
        assert!(ConflictState::Resolved.transition_to(ConflictState::Resolving).is_err());
        // Failed -> Resolving (must go through Detected first)
        assert!(ConflictState::Failed.transition_to(ConflictState::Resolving).is_err());
        // None -> Resolving (must go through Detected first)
        assert!(ConflictState::None.transition_to(ConflictState::Resolving).is_err());
        // None -> Resolved
        assert!(ConflictState::None.transition_to(ConflictState::Resolved).is_err());
        // None -> Failed
        assert!(ConflictState::None.transition_to(ConflictState::Failed).is_err());
        // Resolving -> None
        assert!(ConflictState::Resolving.transition_to(ConflictState::None).is_err());
        // Resolving -> Detected
        assert!(ConflictState::Resolving.transition_to(ConflictState::Detected).is_err());
        // Resolved -> None (must go through Detected)
        assert!(ConflictState::Resolved.transition_to(ConflictState::None).is_err());
        // Resolved -> Failed
        assert!(ConflictState::Resolved.transition_to(ConflictState::Failed).is_err());
    }

    #[test]
    fn test_conflict_state_transition_returns_new_state() {
        let new_state = ConflictState::None
            .transition_to(ConflictState::Detected)
            .expect("valid");
        assert_eq!(new_state, ConflictState::Detected);
    }

    #[test]
    fn test_conflict_state_transition_error_message() {
        let result = ConflictState::None.transition_to(ConflictState::Resolved);
        assert!(result.is_err());
    }

    // ── Conflict construction ────────────────────────────────────────────────

    #[test]
    fn test_conflict_new() {
        let conflict = Conflict::new("feature-branch", "Merge conflict in main.rs");
        assert_eq!(conflict.branch_id, "feature-branch");
        assert_eq!(conflict.state, ConflictState::Detected);
        assert_eq!(conflict.description, "Merge conflict in main.rs");
        assert!(conflict.base_commit.is_none());
        assert!(conflict.conflicting_commits.is_empty());
        assert!(conflict.detected_at.is_some());
        assert!(conflict.resolved_at.is_none());
    }

    #[test]
    fn test_conflict_needs_resolution() {
        let conflict = Conflict::new("branch", "conflict");
        assert!(conflict.needs_resolution());
    }

    #[test]
    fn test_conflict_is_resolved_false_initially() {
        let conflict = Conflict::new("branch", "conflict");
        assert!(!conflict.is_resolved());
    }

    // ── Conflict state transitions via methods ───────────────────────────────

    #[test]
    fn test_conflict_start_resolution() {
        let mut conflict = Conflict::new("branch", "conflict");
        conflict.start_resolution().expect("start resolution");
        assert_eq!(conflict.state, ConflictState::Resolving);
    }

    #[test]
    fn test_conflict_resolve() {
        let mut conflict = Conflict::new("branch", "conflict");
        conflict.start_resolution().expect("start resolution");
        conflict.resolve().expect("resolve");
        assert!(conflict.is_resolved());
        assert!(conflict.resolved_at.is_some());
    }

    #[test]
    fn test_conflict_fail() {
        let mut conflict = Conflict::new("branch", "conflict");
        conflict.start_resolution().expect("start resolution");
        conflict.fail().expect("fail");
        assert_eq!(conflict.state, ConflictState::Failed);
    }

    #[test]
    fn test_conflict_resolve_without_starting_resolution_fails() {
        let mut conflict = Conflict::new("branch", "conflict");
        assert!(conflict.resolve().is_err());
    }

    #[test]
    fn test_conflict_fail_without_starting_resolution_fails() {
        let mut conflict = Conflict::new("branch", "conflict");
        assert!(conflict.fail().is_err());
    }

    #[test]
    fn test_conflict_double_start_resolution_fails() {
        let mut conflict = Conflict::new("branch", "conflict");
        conflict.start_resolution().expect("first");
        assert!(conflict.start_resolution().is_err());
    }

    // ── ConflictManager basic operations ─────────────────────────────────────

    #[test]
    fn test_conflict_manager_new() {
        let manager = ConflictManager::new();
        assert!(manager.is_empty());
        assert_eq!(manager.len(), 0);
    }

    #[test]
    fn test_conflict_manager_default() {
        let manager = ConflictManager::default();
        assert!(manager.is_empty());
    }

    #[test]
    fn test_conflict_manager_register() {
        let mut manager = ConflictManager::new();
        let conflict = Conflict::new("feature", "Merge conflict detected");
        manager
            .register_conflict(conflict)
            .expect("register succeeds");
        assert!(manager.has_conflict("feature"));
        assert_eq!(manager.len(), 1);
    }

    #[test]
    fn test_conflict_manager_register_duplicate_unresolved_fails() {
        let mut manager = ConflictManager::new();
        manager
            .register_conflict(Conflict::new("feature", "conflict 1"))
            .expect("first");
        let result = manager.register_conflict(Conflict::new("feature", "conflict 2"));
        assert!(result.is_err());
    }

    #[test]
    fn test_conflict_manager_register_after_resolution() {
        let mut manager = ConflictManager::new();
        manager
            .register_conflict(Conflict::new("feature", "conflict"))
            .expect("register");
        manager.start_resolution("feature").expect("start");
        manager.resolve("feature").expect("resolve");
        // Now registering a new conflict for same branch should work
        manager
            .register_conflict(Conflict::new("feature", "new conflict"))
            .expect("re-register after resolution");
    }

    #[test]
    fn test_conflict_manager_get_conflict() {
        let mut manager = ConflictManager::new();
        manager
            .register_conflict(Conflict::new("feature", "conflict"))
            .expect("register");
        let conflict = manager.get_conflict("feature");
        assert!(conflict.is_some());
        assert_eq!(conflict.unwrap().branch_id, "feature");
    }

    #[test]
    fn test_conflict_manager_get_conflict_missing() {
        let manager = ConflictManager::new();
        assert!(manager.get_conflict("nonexistent").is_none());
    }

    #[test]
    fn test_conflict_manager_get_conflict_mut() {
        let mut manager = ConflictManager::new();
        manager
            .register_conflict(Conflict::new("feature", "conflict"))
            .expect("register");
        let conflict = manager.get_conflict_mut("feature");
        assert!(conflict.is_some());
        // Mutate through the mutable reference
        conflict.unwrap().description = "updated description".to_string();
        assert_eq!(
            manager.get_conflict("feature").unwrap().description,
            "updated description"
        );
    }

    #[test]
    fn test_conflict_manager_has_conflict() {
        let mut manager = ConflictManager::new();
        assert!(!manager.has_conflict("feature"));
        manager
            .register_conflict(Conflict::new("feature", "conflict"))
            .expect("register");
        assert!(manager.has_conflict("feature"));
        manager.start_resolution("feature").expect("start");
        assert!(manager.has_conflict("feature"));
        manager.resolve("feature").expect("resolve");
        assert!(!manager.has_conflict("feature"));
    }

    // ── ConflictManager resolution flow ──────────────────────────────────────

    #[test]
    fn test_conflict_resolution_flow() {
        let mut manager = ConflictManager::new();
        let conflict = Conflict::new("feature", "Merge conflict detected");
        manager
            .register_conflict(conflict)
            .expect("register succeeds");

        manager
            .start_resolution("feature")
            .expect("start resolution");
        manager.resolve("feature").expect("resolve");

        let conflict = manager.get_conflict("feature").expect("conflict exists");
        assert!(conflict.is_resolved());
    }

    #[test]
    fn test_conflict_manager_start_resolution_missing_fails() {
        let mut manager = ConflictManager::new();
        assert!(manager.start_resolution("nonexistent").is_err());
    }

    #[test]
    fn test_conflict_manager_resolve_missing_fails() {
        let mut manager = ConflictManager::new();
        assert!(manager.resolve("nonexistent").is_err());
    }

    #[test]
    fn test_conflict_manager_fail_missing_fails() {
        let mut manager = ConflictManager::new();
        assert!(manager.fail("nonexistent").is_err());
    }

    #[test]
    fn test_conflict_manager_fail_flow() {
        let mut manager = ConflictManager::new();
        manager
            .register_conflict(Conflict::new("feature", "conflict"))
            .expect("register");
        manager.start_resolution("feature").expect("start");
        manager.fail("feature").expect("fail");
        assert!(!manager.has_conflict("feature"));
    }

    // ── ConflictManager list operations ──────────────────────────────────────

    #[test]
    fn test_unresolved_conflicts() {
        let mut manager = ConflictManager::new();
        let conflict1 = Conflict::new("feature1", "Conflict 1");
        let conflict2 = Conflict::new("feature2", "Conflict 2");
        manager
            .register_conflict(conflict1)
            .expect("register succeeds");
        manager
            .register_conflict(conflict2)
            .expect("register succeeds");

        let unresolved = manager.unresolved_conflicts();
        assert_eq!(unresolved.len(), 2);
    }

    #[test]
    fn test_unresolved_conflicts_after_partial_resolution() {
        let mut manager = ConflictManager::new();
        manager
            .register_conflict(Conflict::new("f1", "c1"))
            .expect("f1");
        manager
            .register_conflict(Conflict::new("f2", "c2"))
            .expect("f2");
        manager.start_resolution("f1").expect("start f1");
        manager.resolve("f1").expect("resolve f1");
        let unresolved = manager.unresolved_conflicts();
        assert_eq!(unresolved.len(), 1);
        assert_eq!(unresolved[0].branch_id, "f2");
    }

    #[test]
    fn test_conflict_manager_remove() {
        let mut manager = ConflictManager::new();
        manager
            .register_conflict(Conflict::new("feature", "conflict"))
            .expect("register");
        let removed = manager.remove("feature");
        assert!(removed.is_some());
        assert!(manager.get_conflict("feature").is_none());
    }

    #[test]
    fn test_conflict_manager_remove_missing() {
        let mut manager = ConflictManager::new();
        let removed = manager.remove("nonexistent");
        assert!(removed.is_none());
    }

    #[test]
    fn test_conflict_manager_clear() {
        let mut manager = ConflictManager::new();
        manager
            .register_conflict(Conflict::new("f1", "c1"))
            .expect("f1");
        manager
            .register_conflict(Conflict::new("f2", "c2"))
            .expect("f2");
        manager.clear();
        assert!(manager.is_empty());
        assert_eq!(manager.len(), 0);
    }

    #[test]
    fn test_conflict_manager_multiple_branches() {
        let mut manager = ConflictManager::new();
        for i in 0..10 {
            manager
                .register_conflict(Conflict::new(format!("branch-{i}"), format!("conflict {i}")))
                .expect("register");
        }
        assert_eq!(manager.len(), 10);
        assert_eq!(manager.unresolved_conflicts().len(), 10);

        // Resolve half
        for i in 0..5 {
            manager.start_resolution(&format!("branch-{i}")).expect("start");
            manager.resolve(&format!("branch-{i}")).expect("resolve");
        }
        assert_eq!(manager.unresolved_conflicts().len(), 5);
        assert_eq!(manager.len(), 10); // All still tracked
    }

    // ── Serde roundtrip tests ──────────────────────────────────────────────────

    #[test]
    fn test_conflict_state_serde_roundtrip_all_variants() {
        for state in [
            ConflictState::None,
            ConflictState::Detected,
            ConflictState::Resolving,
            ConflictState::Resolved,
            ConflictState::Failed,
        ] {
            let json = serde_json::to_string(&state).expect("serialize ok");
            let deserialized: ConflictState = serde_json::from_str(&json).expect("deserialize ok");
            assert_eq!(state, deserialized, "Roundtrip failed for {state:?}");
        }
    }

    #[test]
    fn test_conflict_serde_roundtrip_with_optionals() {
        let mut conflict = Conflict::new("feature-branch", "Merge conflict");
        conflict.base_commit = Some("abc123".to_string());
        conflict.conflicting_commits = vec!["def456".to_string()];

        let json = serde_json::to_string(&conflict).expect("serialize ok");
        let deserialized: Conflict = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(conflict.branch_id, deserialized.branch_id);
        assert_eq!(conflict.state, deserialized.state);
        assert_eq!(conflict.base_commit, deserialized.base_commit);
        assert_eq!(conflict.conflicting_commits, deserialized.conflicting_commits);
    }

    #[test]
    fn test_conflict_serde_with_empty_vecs() {
        let conflict = Conflict::new("branch", "conflict");
        assert!(conflict.conflicting_commits.is_empty());
        assert!(conflict.base_commit.is_none());
        assert!(conflict.detected_at.is_some()); // Conflict::new always sets detected_at

        let json = serde_json::to_string(&conflict).expect("serialize ok");
        let deserialized: Conflict = serde_json::from_str(&json).expect("deserialize ok");
        assert!(deserialized.conflicting_commits.is_empty());
        assert!(deserialized.base_commit.is_none());
    }

    // ── Proptests ────────────────────────────────────────────────────────────

    use proptest::proptest;

    proptest! {
        #[test]
        fn prop_conflict_state_is_terminal_matches_needs_resolution(
            state in 0u8..5u8
        ) {
            let all_states = [
                ConflictState::None,
                ConflictState::Detected,
                ConflictState::Resolving,
                ConflictState::Resolved,
                ConflictState::Failed,
            ];
            let cs = all_states[state as usize];
            // If terminal, should not need resolution (except None which is terminal but also doesn't need resolution)
            // If needs resolution, should not be terminal
            if cs.needs_resolution() {
                assert!(!cs.is_terminal(), "{:?} needs resolution but is terminal", cs);
            }
        }

        #[test]
        fn prop_conflict_new_always_detected(
            branch_id in "[a-z]+",
            description in "[a-zA-Z ]{1,50}"
        ) {
            let conflict = Conflict::new(&branch_id, &description);
            assert_eq!(conflict.state, ConflictState::Detected);
            assert_eq!(conflict.branch_id, branch_id);
            assert_eq!(conflict.description, description);
            assert!(conflict.detected_at.is_some());
            assert!(conflict.resolved_at.is_none());
        }
    }
}
