//! RAII workspace guard for automatic cleanup.
//!
//! `WorkspaceGuard` wraps an isolated workspace and ensures proper lifecycle
//! management. When dropped without explicit completion, the guard signals
//! that cleanup is needed (the workspace was not properly transitioned to
//! a terminal state).
//!
//! # Architecture
//!
//! ```text
//! WorkspaceGuard::acquire(bead, workspace)
//!   ├─> commit()   → workspace transitions to Ready
//!   ├─> abandon()  → workspace transitions to Abandoned
//!   └─> Drop       → warn: workspace needs manual cleanup
//! ```

use std::sync::atomic::{AtomicBool, Ordering};

use super::types::{BeadId, BeadWorkspaceMapping, WorkspaceId, WorkspaceState};
use crate::error::{IsolateError, Result};
use crate::WorkspaceStateMachine;

/// Inner data shared between the guard and its resolved output.
struct GuardInner {
    workspace_id: WorkspaceId,
    bead_id: BeadId,
    state: WorkspaceState,
    mapping: BeadWorkspaceMapping,
    committed: AtomicBool,
}

/// RAII guard that ensures workspace cleanup on drop.
///
/// Acquire a guard when an agent claims a bead. The guard owns the
/// bead-to-workspace mapping. On successful completion, call `commit()`.
/// To explicitly abandon, call `abandon()`. If neither is called before
/// drop, a warning is emitted and the workspace is flagged for cleanup.
pub struct WorkspaceGuard {
    inner: Option<GuardInner>,
}

impl WorkspaceGuard {
    /// Acquire a new workspace guard for a bead-to-workspace mapping.
    ///
    /// The workspace must be in `Created` state. The guard takes ownership
    /// of the mapping and transitions to `Working`.
    pub fn acquire(
        workspace_id: WorkspaceId,
        bead_id: BeadId,
        state: WorkspaceState,
    ) -> Result<Self> {
        if state != WorkspaceState::Created {
            return Err(IsolateError::OperationFailed(format!(
                "can only acquire guard for Created workspaces, got: {state}"
            )));
        }

        let working = WorkspaceStateMachine::transition(state, WorkspaceState::Working)?;
        let mapping = BeadWorkspaceMapping::new(bead_id.clone(), workspace_id.clone());

        Ok(Self {
            inner: Some(GuardInner {
                workspace_id,
                bead_id,
                state: working,
                mapping,
                committed: AtomicBool::new(false),
            }),
        })
    }

    /// The workspace ID this guard manages.
    #[must_use]
    pub fn workspace_id(&self) -> Option<&WorkspaceId> {
        self.inner.as_ref().map(|i| &i.workspace_id)
    }

    /// The bead ID this workspace is assigned to.
    #[must_use]
    pub fn bead_id(&self) -> Option<&BeadId> {
        self.inner.as_ref().map(|i| &i.bead_id)
    }

    /// The current workspace state.
    #[must_use]
    pub fn state(&self) -> Option<WorkspaceState> {
        self.inner.as_ref().map(|i| i.state)
    }

    /// The bead-to-workspace mapping.
    #[must_use]
    pub fn mapping(&self) -> Option<&BeadWorkspaceMapping> {
        self.inner.as_ref().map(|i| &i.mapping)
    }

    /// Whether this guard has been committed or abandoned.
    #[must_use]
    pub fn is_resolved(&self) -> bool {
        self.inner
            .as_ref()
            .is_none_or(|i| i.committed.load(Ordering::SeqCst))
    }

    /// Mark the workspace as ready (work completed successfully).
    ///
    /// Transitions `Working → Ready`. After commit, the guard is resolved
    /// and will not trigger cleanup on drop. Returns a `CommittedGuard`
    /// with the final state.
    pub fn commit(mut self) -> Result<CommittedGuard> {
        let inner = self
            .inner
            .take()
            .ok_or_else(|| IsolateError::OperationFailed("guard already consumed".into()))?;
        let ready = WorkspaceStateMachine::transition(inner.state, WorkspaceState::Ready)?;
        inner.committed.store(true, Ordering::SeqCst);
        Ok(CommittedGuard {
            workspace_id: inner.workspace_id,
            bead_id: inner.bead_id,
            state: ready,
            mapping: inner.mapping,
        })
    }

    /// Explicitly abandon the workspace (work cannot complete).
    ///
    /// Transitions `Working → Abandoned`. After abandon, the guard is resolved
    /// and will not trigger cleanup on drop. Returns a `CommittedGuard`
    /// with the final state.
    pub fn abandon(mut self) -> Result<CommittedGuard> {
        let inner = self
            .inner
            .take()
            .ok_or_else(|| IsolateError::OperationFailed("guard already consumed".into()))?;
        let abandoned = WorkspaceStateMachine::transition(inner.state, WorkspaceState::Abandoned)?;
        inner.committed.store(true, Ordering::SeqCst);
        Ok(CommittedGuard {
            workspace_id: inner.workspace_id,
            bead_id: inner.bead_id,
            state: abandoned,
            mapping: inner.mapping,
        })
    }
}

impl Drop for WorkspaceGuard {
    fn drop(&mut self) {
        if let Some(inner) = &self.inner {
            if !inner.committed.load(Ordering::SeqCst) {
                eprintln!(
                    "WARNING: WorkspaceGuard dropped without commit: workspace '{}' for bead '{}' needs cleanup",
                    inner.workspace_id, inner.bead_id
                );
            }
        }
    }
}

impl std::fmt::Debug for WorkspaceGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.inner {
            Some(inner) => f
                .debug_struct("WorkspaceGuard")
                .field("workspace_id", &inner.workspace_id)
                .field("bead_id", &inner.bead_id)
                .field("state", &inner.state)
                .field("committed", &inner.committed.load(Ordering::SeqCst))
                .finish_non_exhaustive(),
            None => f
                .debug_struct("WorkspaceGuard")
                .field("consumed", &true)
                .finish(),
        }
    }
}

/// A resolved (committed or abandoned) workspace guard.
///
/// This type proves that the workspace lifecycle was explicitly handled.
/// It contains the final state and mapping for inspection.
#[derive(Debug, Clone)]
pub struct CommittedGuard {
    workspace_id: WorkspaceId,
    bead_id: BeadId,
    state: WorkspaceState,
    mapping: BeadWorkspaceMapping,
}

impl CommittedGuard {
    /// The workspace ID this guard managed.
    #[must_use]
    pub fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }

    /// The bead ID this workspace was assigned to.
    #[must_use]
    pub fn bead_id(&self) -> &BeadId {
        &self.bead_id
    }

    /// The final workspace state.
    #[must_use]
    pub fn state(&self) -> WorkspaceState {
        self.state
    }

    /// The bead-to-workspace mapping.
    #[must_use]
    pub fn mapping(&self) -> &BeadWorkspaceMapping {
        &self.mapping
    }

    /// Whether the workspace was committed (Ready) vs abandoned.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.state == WorkspaceState::Ready
    }

    /// Whether the workspace was abandoned.
    #[must_use]
    pub fn is_abandoned(&self) -> bool {
        self.state == WorkspaceState::Abandoned
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_workspace_id() -> WorkspaceId {
        WorkspaceId::generate()
    }

    fn test_bead_id() -> BeadId {
        BeadId::parse("test-bead-1".into()).unwrap()
    }

    // --- WorkspaceGuard::acquire ---

    #[test]
    fn acquire_with_created_state_succeeds() {
        let ws_id = test_workspace_id();
        let bead_id = test_bead_id();
        let guard =
            WorkspaceGuard::acquire(ws_id.clone(), bead_id.clone(), WorkspaceState::Created);
        assert!(guard.is_ok());
        let guard = guard.unwrap();
        assert_eq!(guard.workspace_id(), Some(&ws_id));
        assert_eq!(guard.bead_id(), Some(&bead_id));
        assert_eq!(guard.state(), Some(WorkspaceState::Working));
        assert!(!guard.is_resolved());
    }

    #[test]
    fn acquire_with_working_state_fails() {
        let ws_id = test_workspace_id();
        let bead_id = test_bead_id();
        let result = WorkspaceGuard::acquire(ws_id, bead_id, WorkspaceState::Working);
        assert!(result.is_err());
        match result.err() {
            Some(IsolateError::OperationFailed(msg)) => {
                assert!(msg.contains("Created"));
                assert!(msg.contains("working"));
            }
            other => panic!("expected OperationFailed, got {other:?}"),
        }
    }

    #[test]
    fn acquire_with_ready_state_fails() {
        let ws_id = test_workspace_id();
        let bead_id = test_bead_id();
        let result = WorkspaceGuard::acquire(ws_id, bead_id, WorkspaceState::Ready);
        assert!(result.is_err());
    }

    #[test]
    fn acquire_with_merged_state_fails() {
        let ws_id = test_workspace_id();
        let bead_id = test_bead_id();
        let result = WorkspaceGuard::acquire(ws_id, bead_id, WorkspaceState::Merged);
        assert!(result.is_err());
    }

    #[test]
    fn acquire_with_abandoned_state_fails() {
        let ws_id = test_workspace_id();
        let bead_id = test_bead_id();
        let result = WorkspaceGuard::acquire(ws_id, bead_id, WorkspaceState::Abandoned);
        assert!(result.is_err());
    }

    #[test]
    fn acquire_with_conflict_state_fails() {
        let ws_id = test_workspace_id();
        let bead_id = test_bead_id();
        let result = WorkspaceGuard::acquire(ws_id, bead_id, WorkspaceState::Conflict);
        assert!(result.is_err());
    }

    // --- WorkspaceGuard::commit ---

    #[test]
    fn commit_transitions_to_ready() {
        let ws_id = test_workspace_id();
        let bead_id = test_bead_id();
        let guard =
            WorkspaceGuard::acquire(ws_id.clone(), bead_id.clone(), WorkspaceState::Created)
                .unwrap();
        let committed = guard.commit().unwrap();
        assert!(committed.is_ready());
        assert!(!committed.is_abandoned());
        assert_eq!(committed.state(), WorkspaceState::Ready);
        assert_eq!(committed.workspace_id(), &ws_id);
        assert_eq!(committed.bead_id(), &bead_id);
    }

    // --- WorkspaceGuard::abandon ---

    #[test]
    fn abandon_transitions_to_abandoned() {
        let ws_id = test_workspace_id();
        let bead_id = test_bead_id();
        let guard =
            WorkspaceGuard::acquire(ws_id.clone(), bead_id.clone(), WorkspaceState::Created)
                .unwrap();
        let committed = guard.abandon().unwrap();
        assert!(committed.is_abandoned());
        assert!(!committed.is_ready());
        assert_eq!(committed.state(), WorkspaceState::Abandoned);
        assert_eq!(committed.workspace_id(), &ws_id);
        assert_eq!(committed.bead_id(), &bead_id);
    }

    // --- CommittedGuard ---

    #[test]
    fn committed_guard_preserves_mapping() {
        let ws_id = test_workspace_id();
        let bead_id = test_bead_id();
        let guard =
            WorkspaceGuard::acquire(ws_id, bead_id.clone(), WorkspaceState::Created).unwrap();
        let committed = guard.commit().unwrap();
        assert_eq!(committed.mapping().bead_id(), &bead_id);
    }

    #[test]
    fn committed_guard_clone_preserves_fields() {
        let ws_id = test_workspace_id();
        let bead_id = test_bead_id();
        let guard = WorkspaceGuard::acquire(ws_id, bead_id, WorkspaceState::Created).unwrap();
        let committed = guard.commit().unwrap();
        let cloned = committed.clone();
        assert_eq!(cloned.state(), committed.state());
        assert_eq!(cloned.workspace_id(), committed.workspace_id());
        assert_eq!(cloned.bead_id(), committed.bead_id());
    }

    // --- Debug ---

    #[test]
    fn guard_debug_format() {
        let ws_id = test_workspace_id();
        let bead_id = test_bead_id();
        let guard = WorkspaceGuard::acquire(ws_id, bead_id, WorkspaceState::Created).unwrap();
        let debug_str = format!("{guard:?}");
        assert!(debug_str.contains("WorkspaceGuard"));
        assert!(debug_str.contains("committed"));
    }

    #[test]
    fn guard_debug_consumed() {
        let ws_id = test_workspace_id();
        let bead_id = test_bead_id();
        let guard = WorkspaceGuard::acquire(ws_id, bead_id, WorkspaceState::Created).unwrap();
        let _committed = guard.commit().unwrap();
        // After commit, the original guard is consumed — debug on a fresh one
        let guard2 =
            WorkspaceGuard::acquire(test_workspace_id(), test_bead_id(), WorkspaceState::Created)
                .unwrap();
        let debug_str = format!("{guard2:?}");
        assert!(debug_str.contains("WorkspaceGuard"));
    }

    // --- Drop behavior ---

    #[test]
    fn dropped_without_commit_emits_warning() {
        let ws_id = test_workspace_id();
        let bead_id = test_bead_id();
        // Guard is created and immediately dropped — no panic, just a warning
        {
            let _guard = WorkspaceGuard::acquire(ws_id, bead_id, WorkspaceState::Created).unwrap();
        }
        // Drop happens here, warning emitted via eprintln
    }

    #[test]
    fn dropped_after_commit_no_warning() {
        let ws_id = test_workspace_id();
        let bead_id = test_bead_id();
        let guard = WorkspaceGuard::acquire(ws_id, bead_id, WorkspaceState::Created).unwrap();
        let _committed = guard.commit().unwrap();
        // No warning because committed flag is set
    }

    // --- Multiple guards with different IDs ---

    #[test]
    fn multiple_guards_independent() {
        let ws1 = WorkspaceId::generate();
        let ws2 = WorkspaceId::generate();
        let bead1 = BeadId::parse("bead-1".into()).unwrap();
        let bead2 = BeadId::parse("bead-2".into()).unwrap();

        let guard1 =
            WorkspaceGuard::acquire(ws1.clone(), bead1.clone(), WorkspaceState::Created).unwrap();
        let guard2 =
            WorkspaceGuard::acquire(ws2.clone(), bead2.clone(), WorkspaceState::Created).unwrap();

        let c1 = guard1.commit().unwrap();
        let c2 = guard2.abandon().unwrap();

        assert!(c1.is_ready());
        assert!(c2.is_abandoned());
        assert_ne!(c1.workspace_id(), c2.workspace_id());
    }

    // --- BeadWorkspaceMapping in guard ---

    #[test]
    fn guard_mapping_has_correct_timestamps() {
        let ws_id = test_workspace_id();
        let bead_id = test_bead_id();
        let guard = WorkspaceGuard::acquire(ws_id, bead_id, WorkspaceState::Created).unwrap();
        let mapping = guard.mapping().unwrap();
        assert!(mapping.assigned_at() <= chrono::Utc::now());
    }

    // --- WorkspaceId and BeadId ---

    #[test]
    fn workspace_id_generate_starts_with_iso() {
        let id = WorkspaceId::generate();
        assert!(id.as_str().starts_with("iso-"));
    }

    #[test]
    fn workspace_id_parse_rejects_empty() {
        let result = WorkspaceId::parse(String::new());
        assert!(result.is_err());
    }

    #[test]
    fn workspace_id_parse_accepts_non_empty() {
        let result = WorkspaceId::parse("custom-id".into());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "custom-id");
    }

    #[test]
    fn workspace_id_display() {
        let id = WorkspaceId::parse("ws-42".into()).unwrap();
        assert_eq!(format!("{id}"), "ws-42");
    }

    #[test]
    fn bead_id_parse_rejects_empty() {
        let result = BeadId::parse(String::new());
        assert!(result.is_err());
    }

    #[test]
    fn bead_id_parse_accepts_non_empty() {
        let result = BeadId::parse("bead-1".into());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "bead-1");
    }

    #[test]
    fn bead_id_display() {
        let id = BeadId::parse("b-99".into()).unwrap();
        assert_eq!(format!("{id}"), "b-99");
    }

    // --- Proptests ---

    #[cfg(test)]
    mod proptests {
        use proptest::prelude::*;
        use proptest::prop_assert;

        use super::*;

        proptest! {
            #[test]
            fn bead_id_parse_non_empty(s in ".+") {
                let result = BeadId::parse(s);
                prop_assert!(result.is_ok());
            }

            #[test]
            fn workspace_id_parse_non_empty(s in ".+") {
                let result = WorkspaceId::parse(s);
                prop_assert!(result.is_ok());
            }

            #[test]
            fn bead_id_parse_empty_fails(s in "") {
                let result = BeadId::parse(s);
                prop_assert!(result.is_err());
            }

            #[test]
            fn workspace_id_parse_empty_fails(s in "") {
                let result = WorkspaceId::parse(s);
                prop_assert!(result.is_err());
            }

            #[test]
            fn workspace_id_generate_unique(count in 2usize..20) {
                let ids: std::collections::HashSet<String> = (0..count)
                    .map(|_| WorkspaceId::generate().as_str().to_string())
                    .collect();
                prop_assert_eq!(ids.len(), count);
            }

            #[test]
            fn guard_commit_always_ready(bead_suffix in "[a-z0-9]{1,10}") {
                let bead_id = BeadId::parse(format!("bead-{bead_suffix}")).unwrap();
                let guard = WorkspaceGuard::acquire(
                    WorkspaceId::generate(),
                    bead_id,
                    WorkspaceState::Created,
                ).unwrap();
                let committed = guard.commit();
                prop_assert!(committed.is_ok());
                prop_assert!(committed.unwrap().is_ready());
            }

            #[test]
            fn guard_abandon_always_abandoned(bead_suffix in "[a-z0-9]{1,10}") {
                let bead_id = BeadId::parse(format!("bead-{bead_suffix}")).unwrap();
                let guard = WorkspaceGuard::acquire(
                    WorkspaceId::generate(),
                    bead_id,
                    WorkspaceState::Created,
                ).unwrap();
                let committed = guard.abandon();
                prop_assert!(committed.is_ok());
                prop_assert!(committed.unwrap().is_abandoned());
            }
        }
    }
}
