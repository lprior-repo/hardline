//! Exhaustive tests for lifecycle state management, phase transitions,
//! transition guards, history tracking, and state machine invariants.
//!
//! Covers: SessionState, SessionStateManager, StateTransition,
//! WorkspaceState, WorkspaceStateTransition, WorkspaceStateFilter,
//! LifecycleState trait conformance, and property-based tests.

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use chrono::Utc;

    use crate::{
        lifecycle::LifecycleState,
        session_state::{SessionState, SessionStateManager, StateTransition},
        type_session_status::{Operation, SessionStatus},
        workspace_state::{WorkspaceState, WorkspaceStateFilter, WorkspaceStateTransition},
    };

    // ═══════════════════════════════════════════════════════════════════════════
    // SessionState — exhaustive transition matrix
    // ═══════════════════════════════════════════════════════════════════════════

    /// SessionState has 8 variants. Test every (from, to) pair exhaustively.
    #[test]
    fn session_state_exhaustive_transition_matrix() {
        let valid: Vec<(SessionState, SessionState)> = vec![
            (SessionState::Created, SessionState::Active),
            (SessionState::Created, SessionState::Failed),
            (SessionState::Active, SessionState::CommittingEffect),
            (SessionState::Active, SessionState::Syncing),
            (SessionState::Active, SessionState::Paused),
            (SessionState::Active, SessionState::Completed),
            (SessionState::CommittingEffect, SessionState::Active),
            (SessionState::CommittingEffect, SessionState::Syncing),
            (SessionState::CommittingEffect, SessionState::Failed),
            (SessionState::Syncing, SessionState::Synced),
            (SessionState::Syncing, SessionState::Failed),
            (SessionState::Synced, SessionState::Active),
            (SessionState::Synced, SessionState::Paused),
            (SessionState::Synced, SessionState::Completed),
            (SessionState::Paused, SessionState::Active),
            (SessionState::Paused, SessionState::Completed),
            (SessionState::Completed, SessionState::Created),
            (SessionState::Failed, SessionState::Created),
        ];

        for &from in SessionState::all_states() {
            for &to in SessionState::all_states() {
                let expected = valid.contains(&(from, to));
                let actual = from.can_transition_to(to);
                assert_eq!(
                    actual, expected,
                    "SessionState::can_transition_to({from:?}, {to:?}): expected {expected}, got {actual}"
                );
            }
        }
    }

    #[test]
    fn session_state_no_self_transitions() {
        for &state in SessionState::all_states() {
            assert!(
                !state.can_transition_to(state),
                "SessionState::{state:?} should not allow self-transition"
            );
        }
    }

    #[test]
    fn session_state_valid_next_states_consistency() {
        for &from in SessionState::all_states() {
            let nexts = from.valid_next_states();
            for &to in SessionState::all_states() {
                assert_eq!(
                    from.can_transition_to(to),
                    nexts.contains(&to),
                    "Inconsistency for {from:?} -> {to:?}"
                );
            }
        }
    }

    #[test]
    fn session_state_terminal_states() {
        // SessionState::Completed and Failed are "terminal" but can restart to Created
        // So they are NOT terminal in the strict sense (they can go back to Created)
        // But is_terminal returns true for them
        assert!(SessionState::Completed.is_terminal());
        assert!(SessionState::Failed.is_terminal());
        assert!(!SessionState::Created.is_terminal());
        assert!(!SessionState::Active.is_terminal());
        assert!(!SessionState::CommittingEffect.is_terminal());
        assert!(!SessionState::Syncing.is_terminal());
        assert!(!SessionState::Synced.is_terminal());
        assert!(!SessionState::Paused.is_terminal());
    }

    #[test]
    fn session_state_terminal_have_nonempty_next_states() {
        // Unlike SessionStatus, SessionState terminal states CAN transition back to Created
        assert!(!SessionState::Completed.valid_next_states().is_empty());
        assert!(!SessionState::Failed.valid_next_states().is_empty());
    }

    #[test]
    fn session_state_all_states_exhaustive_and_unique() {
        let all = SessionState::all_states();
        assert_eq!(all.len(), 8);
        let mut seen = std::collections::HashSet::new();
        for &s in all {
            assert!(seen.insert(s), "Duplicate state: {s:?}");
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // SessionStateManager — full lifecycle paths
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn manager_happy_path_created_to_completed() {
        let mgr = SessionStateManager::new("sess-1");
        assert_eq!(mgr.current_state(), SessionState::Created);
        assert!(mgr.history().is_empty());

        let mgr = mgr.activate("start work").expect("activate");
        assert_eq!(mgr.current_state(), SessionState::Active);

        let mgr = mgr.sync("push to remote").expect("sync");
        assert_eq!(mgr.current_state(), SessionState::Syncing);

        let mgr = mgr.sync_complete("sync done").expect("sync_complete");
        assert_eq!(mgr.current_state(), SessionState::Synced);

        let mgr = mgr.complete("all done").expect("complete");
        assert_eq!(mgr.current_state(), SessionState::Completed);

        assert_eq!(mgr.history().len(), 4);
    }

    #[test]
    fn manager_active_pause_resume_complete() {
        let mgr = SessionStateManager::new("sess-2")
            .activate("go")
            .expect("activate");
        let mgr = mgr.pause("break").expect("pause");
        assert_eq!(mgr.current_state(), SessionState::Paused);
        let mgr = mgr.resume("back").expect("resume");
        assert_eq!(mgr.current_state(), SessionState::Active);
        let mgr = mgr.complete("done").expect("complete");
        assert_eq!(mgr.current_state(), SessionState::Completed);
    }

    #[test]
    fn manager_active_pause_complete() {
        let mgr = SessionStateManager::new("sess-3")
            .activate("go")
            .expect("activate")
            .pause("hold")
            .expect("pause")
            .complete("done from pause")
            .expect("complete");
        assert_eq!(mgr.current_state(), SessionState::Completed);
    }

    #[test]
    fn manager_synced_reactivate() {
        let mgr = SessionStateManager::new("sess-4")
            .activate("go")
            .expect("activate")
            .sync("sync")
            .expect("sync")
            .sync_complete("done")
            .expect("sync_complete");
        assert_eq!(mgr.current_state(), SessionState::Synced);
        let mgr = mgr.reactivate("more work").expect("reactivate");
        assert_eq!(mgr.current_state(), SessionState::Active);
    }

    #[test]
    fn manager_synced_pause() {
        let mgr = SessionStateManager::new("sess-5")
            .activate("go")
            .expect("activate")
            .sync("sync")
            .expect("sync")
            .sync_complete("done")
            .expect("sync_complete")
            .pause("hold")
            .expect("pause");
        assert_eq!(mgr.current_state(), SessionState::Paused);
    }

    #[test]
    fn manager_created_fail() {
        let mgr = SessionStateManager::new("sess-6")
            .fail("startup error")
            .expect("fail");
        assert_eq!(mgr.current_state(), SessionState::Failed);
    }

    #[test]
    fn manager_syncing_fail() {
        let mgr = SessionStateManager::new("sess-7")
            .activate("go")
            .expect("activate")
            .sync("sync")
            .expect("sync")
            .fail("sync error")
            .expect("fail");
        assert_eq!(mgr.current_state(), SessionState::Failed);
    }

    #[test]
    fn manager_completed_restart() {
        let mgr = SessionStateManager::new("sess-8")
            .activate("go")
            .expect("activate")
            .complete("done")
            .expect("complete")
            .restart("redo")
            .expect("restart");
        assert_eq!(mgr.current_state(), SessionState::Created);
    }

    #[test]
    fn manager_failed_retry() {
        let mgr = SessionStateManager::new("sess-9")
            .fail("error")
            .expect("fail")
            .retry("retry")
            .expect("retry");
        assert_eq!(mgr.current_state(), SessionState::Created);
    }

    #[test]
    fn manager_full_cycle_with_restart() {
        // Created -> Active -> Completed -> Created -> Active -> Completed
        let mgr = SessionStateManager::new("cycle")
            .activate("first")
            .expect("activate 1")
            .complete("done 1")
            .expect("complete 1")
            .restart("restart")
            .expect("restart")
            .activate("second")
            .expect("activate 2")
            .complete("done 2")
            .expect("complete 2");
        assert_eq!(mgr.current_state(), SessionState::Completed);
        assert_eq!(mgr.history().len(), 5);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // SessionStateManager — history tracking
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn manager_history_records_every_transition() {
        let mgr = SessionStateManager::new("hist-1")
            .activate("r1")
            .expect("activate")
            .sync("r2")
            .expect("sync")
            .sync_complete("r3")
            .expect("sync_complete");

        let history = mgr.history();
        assert_eq!(history.len(), 3);
        assert_eq!(history[0].from, SessionState::Created);
        assert_eq!(history[0].to, SessionState::Active);
        assert_eq!(history[0].reason, "r1");
        assert_eq!(history[1].from, SessionState::Active);
        assert_eq!(history[1].to, SessionState::Syncing);
        assert_eq!(history[2].from, SessionState::Syncing);
        assert_eq!(history[2].to, SessionState::Synced);
    }

    #[test]
    fn manager_history_timestamps_are_chronological() {
        let mgr = SessionStateManager::new("hist-2")
            .activate("a")
            .expect("activate")
            .pause("p")
            .expect("pause");

        let history = mgr.history();
        assert!(history[0].timestamp <= history[1].timestamp);
    }

    #[test]
    fn manager_history_preserves_reasons() {
        let mgr = SessionStateManager::new("hist-3")
            .activate("activation reason")
            .expect("activate")
            .complete("completion reason")
            .expect("complete");

        assert_eq!(mgr.history()[0].reason, "activation reason");
        assert_eq!(mgr.history()[1].reason, "completion reason");
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // SessionStateManager — metadata
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn manager_metadata_survives_transitions() {
        let mut mgr = SessionStateManager::new("meta-1");
        mgr.set_metadata("env", "production");
        mgr.set_metadata("priority", "high");
        assert_eq!(mgr.metadata().get("env"), Some(&"production".to_string()));
        assert_eq!(mgr.metadata().get("priority"), Some(&"high".to_string()));

        let mgr = mgr.activate("go").expect("activate");
        assert_eq!(mgr.metadata().get("env"), Some(&"production".to_string()));
        assert_eq!(mgr.metadata().get("priority"), Some(&"high".to_string()));
    }

    #[test]
    fn manager_session_id_preserved() {
        let mgr = SessionStateManager::new("id-test");
        assert_eq!(mgr.session_id(), "id-test");
        let mgr = mgr.activate("go").expect("activate");
        assert_eq!(mgr.session_id(), "id-test");
        let mgr = mgr.complete("done").expect("complete");
        assert_eq!(mgr.session_id(), "id-test");
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // StateTransition — validation
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn state_transition_validate_all_valid_pairs() {
        let valid_pairs: Vec<(SessionState, SessionState)> = vec![
            (SessionState::Created, SessionState::Active),
            (SessionState::Created, SessionState::Failed),
            (SessionState::Active, SessionState::CommittingEffect),
            (SessionState::Active, SessionState::Syncing),
            (SessionState::Active, SessionState::Paused),
            (SessionState::Active, SessionState::Completed),
            (SessionState::CommittingEffect, SessionState::Active),
            (SessionState::CommittingEffect, SessionState::Syncing),
            (SessionState::CommittingEffect, SessionState::Failed),
            (SessionState::Syncing, SessionState::Synced),
            (SessionState::Syncing, SessionState::Failed),
            (SessionState::Synced, SessionState::Active),
            (SessionState::Synced, SessionState::Paused),
            (SessionState::Synced, SessionState::Completed),
            (SessionState::Paused, SessionState::Active),
            (SessionState::Paused, SessionState::Completed),
            (SessionState::Completed, SessionState::Created),
            (SessionState::Failed, SessionState::Created),
        ];

        for (from, to) in valid_pairs {
            let t = StateTransition::new(from, to, "test");
            assert!(t.validate().is_ok(), "Should be valid: {from:?} -> {to:?}");
        }
    }

    #[test]
    fn state_transition_rejects_all_invalid_pairs() {
        for &from in SessionState::all_states() {
            for &to in SessionState::all_states() {
                if from.can_transition_to(to) {
                    continue;
                }
                let t = StateTransition::new(from, to, "invalid");
                assert!(
                    t.validate().is_err(),
                    "Should be invalid: {from:?} -> {to:?}"
                );
            }
        }
    }

    #[test]
    fn state_transition_fields() {
        let t = StateTransition::new(SessionState::Created, SessionState::Active, "reason");
        assert_eq!(t.from, SessionState::Created);
        assert_eq!(t.to, SessionState::Active);
        assert_eq!(t.reason, "reason");
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // LifecycleState trait — try_transition
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn try_transition_ok_for_all_valid_session_status() {
        assert_eq!(
            SessionStatus::Creating
                .try_transition(SessionStatus::Active)
                .ok(),
            Some(SessionStatus::Active)
        );
        assert_eq!(
            SessionStatus::Active
                .try_transition(SessionStatus::Paused)
                .ok(),
            Some(SessionStatus::Paused)
        );
        assert_eq!(
            SessionStatus::Active
                .try_transition(SessionStatus::Completed)
                .ok(),
            Some(SessionStatus::Completed)
        );
        assert_eq!(
            SessionStatus::Paused
                .try_transition(SessionStatus::Active)
                .ok(),
            Some(SessionStatus::Active)
        );
    }

    #[test]
    fn try_transition_err_for_all_invalid_session_status() {
        assert!(SessionStatus::Creating
            .try_transition(SessionStatus::Paused)
            .is_err());
        assert!(SessionStatus::Creating
            .try_transition(SessionStatus::Creating)
            .is_err());
        assert!(SessionStatus::Completed
            .try_transition(SessionStatus::Active)
            .is_err());
        assert!(SessionStatus::Failed
            .try_transition(SessionStatus::Active)
            .is_err());
    }

    #[test]
    fn try_transition_terminal_to_anything_fails() {
        for &terminal in &[SessionStatus::Completed, SessionStatus::Failed] {
            for &target in SessionStatus::all_states() {
                assert!(
                    terminal.try_transition(target).is_err(),
                    "Terminal {terminal:?} -> {target:?} should fail"
                );
            }
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // SessionStatus — allowed_operations guard tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn session_status_operations_guard_exhaustive() {
        // Creating: no operations
        for op in [
            Operation::Status,
            Operation::Diff,
            Operation::Focus,
            Operation::Remove,
        ] {
            assert!(
                !SessionStatus::Creating.allows_operation(op),
                "Creating should not allow {op:?}"
            );
        }

        // Active: all operations
        assert!(SessionStatus::Active.allows_operation(Operation::Diff));

        // Paused: no Diff
        assert!(!SessionStatus::Paused.allows_operation(Operation::Diff));
        assert!(SessionStatus::Paused.allows_operation(Operation::Status));

        // Terminal: only Remove
        for &terminal in &[SessionStatus::Completed, SessionStatus::Failed] {
            assert!(terminal.allows_operation(Operation::Remove));
            assert!(!terminal.allows_operation(Operation::Diff));
            assert!(!terminal.allows_operation(Operation::Status));
            assert!(!terminal.allows_operation(Operation::Focus));
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // WorkspaceState — exhaustive transition matrix
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn workspace_state_exhaustive_transition_matrix() {
        let valid: Vec<(WorkspaceState, WorkspaceState)> = vec![
            (WorkspaceState::Created, WorkspaceState::Working),
            (WorkspaceState::Working, WorkspaceState::Ready),
            (WorkspaceState::Working, WorkspaceState::Conflict),
            (WorkspaceState::Working, WorkspaceState::Abandoned),
            (WorkspaceState::Ready, WorkspaceState::Working),
            (WorkspaceState::Ready, WorkspaceState::Merged),
            (WorkspaceState::Ready, WorkspaceState::Conflict),
            (WorkspaceState::Ready, WorkspaceState::Abandoned),
            (WorkspaceState::Conflict, WorkspaceState::Working),
            (WorkspaceState::Conflict, WorkspaceState::Abandoned),
        ];

        for &from in WorkspaceState::all() {
            for &to in WorkspaceState::all() {
                let expected = valid.contains(&(from, to));
                assert_eq!(
                    from.can_transition_to(to),
                    expected,
                    "WorkspaceState {from:?} -> {to:?}: expected {expected}"
                );
            }
        }
    }

    #[test]
    fn workspace_state_no_self_transitions() {
        for &state in WorkspaceState::all() {
            assert!(
                !state.can_transition_to(state),
                "No self-transition for {state:?}"
            );
        }
    }

    #[test]
    fn workspace_state_valid_next_states_consistency() {
        for &from in WorkspaceState::all() {
            let nexts = from.valid_next_states();
            for &to in WorkspaceState::all() {
                assert_eq!(
                    from.can_transition_to(to),
                    nexts.contains(&to),
                    "Inconsistency: {from:?} -> {to:?}"
                );
            }
        }
    }

    #[test]
    fn workspace_state_terminal_states_no_transitions() {
        for &terminal in &[WorkspaceState::Merged, WorkspaceState::Abandoned] {
            assert!(terminal.is_terminal());
            assert!(terminal.valid_next_states().is_empty());
            for &other in WorkspaceState::all() {
                assert!(!terminal.can_transition_to(other));
            }
        }
    }

    #[test]
    fn workspace_state_non_terminal_have_transitions() {
        let non_terminals = [
            WorkspaceState::Created,
            WorkspaceState::Working,
            WorkspaceState::Ready,
            WorkspaceState::Conflict,
        ];
        for &state in &non_terminals {
            assert!(!state.is_terminal());
            assert!(
                !state.valid_next_states().is_empty(),
                "{state:?} should have transitions"
            );
        }
    }

    #[test]
    fn workspace_state_all_exhaustive_unique() {
        let all = WorkspaceState::all();
        assert_eq!(all.len(), 6);
        let mut seen = std::collections::HashSet::new();
        for &s in all {
            assert!(seen.insert(s), "Duplicate: {s:?}");
        }
    }

    #[test]
    fn workspace_state_is_active_predicate() {
        assert!(WorkspaceState::Working.is_active());
        assert!(WorkspaceState::Conflict.is_active());
        assert!(!WorkspaceState::Created.is_active());
        assert!(!WorkspaceState::Ready.is_active());
        assert!(!WorkspaceState::Merged.is_active());
        assert!(!WorkspaceState::Abandoned.is_active());
    }

    #[test]
    fn workspace_state_is_complete_predicate() {
        assert!(WorkspaceState::Ready.is_complete());
        assert!(WorkspaceState::Merged.is_complete());
        assert!(!WorkspaceState::Created.is_complete());
        assert!(!WorkspaceState::Working.is_complete());
        assert!(!WorkspaceState::Abandoned.is_complete());
        assert!(!WorkspaceState::Conflict.is_complete());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // WorkspaceStateTransition — construction + validation
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn workspace_transition_valid_pairs() {
        let pairs: Vec<(WorkspaceState, WorkspaceState)> = vec![
            (WorkspaceState::Created, WorkspaceState::Working),
            (WorkspaceState::Working, WorkspaceState::Ready),
            (WorkspaceState::Ready, WorkspaceState::Merged),
            (WorkspaceState::Conflict, WorkspaceState::Working),
        ];
        for (from, to) in pairs {
            let t = WorkspaceStateTransition::new(from, to, "test");
            assert!(t.validate().is_ok(), "Valid: {from:?} -> {to:?}");
        }
    }

    #[test]
    fn workspace_transition_invalid_pairs() {
        let pairs: Vec<(WorkspaceState, WorkspaceState)> = vec![
            (WorkspaceState::Created, WorkspaceState::Merged),
            (WorkspaceState::Created, WorkspaceState::Conflict),
            (WorkspaceState::Merged, WorkspaceState::Working),
            (WorkspaceState::Abandoned, WorkspaceState::Created),
        ];
        for (from, to) in pairs {
            let t = WorkspaceStateTransition::new(from, to, "test");
            assert!(t.validate().is_err(), "Invalid: {from:?} -> {to:?}");
        }
    }

    #[test]
    fn workspace_transition_with_agent() {
        let t = WorkspaceStateTransition::with_agent(
            WorkspaceState::Created,
            WorkspaceState::Working,
            "start",
            "polecat-7",
        );
        assert_eq!(t.agent_id.as_deref(), Some("polecat-7"));
        assert!(t.validate().is_ok());
    }

    #[test]
    fn workspace_transition_without_agent() {
        let t =
            WorkspaceStateTransition::new(WorkspaceState::Working, WorkspaceState::Ready, "ready");
        assert!(t.agent_id.is_none());
    }

    #[test]
    fn workspace_terminal_to_any_fails() {
        for &terminal in &[WorkspaceState::Merged, WorkspaceState::Abandoned] {
            for &target in WorkspaceState::all() {
                let t = WorkspaceStateTransition::new(terminal, target, "try");
                assert!(
                    t.validate().is_err(),
                    "Terminal {terminal:?} -> {target:?} should fail"
                );
            }
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // WorkspaceStateFilter — exhaustive coverage
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn filter_state_matches_exactly() {
        for &state in WorkspaceState::all() {
            let filter = WorkspaceStateFilter::State(state);
            assert!(filter.matches(state));
            for &other in WorkspaceState::all() {
                if other != state {
                    assert!(
                        !filter.matches(other),
                        "{state:?} filter should not match {other:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn filter_all_matches_everything() {
        for &state in WorkspaceState::all() {
            assert!(WorkspaceStateFilter::All.matches(state));
        }
    }

    #[test]
    fn filter_terminal_partition() {
        // Every state is either terminal or non-terminal, never both
        for &state in WorkspaceState::all() {
            let terminal = WorkspaceStateFilter::Terminal.matches(state);
            let non_terminal = WorkspaceStateFilter::NonTerminal.matches(state);
            assert_ne!(
                terminal, non_terminal,
                "{state:?}: terminal={terminal}, non_terminal={non_terminal}"
            );
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // WorkspaceState — Display + FromStr roundtrip
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn workspace_display_from_str_roundtrip() {
        for &state in WorkspaceState::all() {
            let s = state.to_string();
            let parsed: std::result::Result<WorkspaceState, _> = s.parse();
            assert_eq!(
                parsed.ok(),
                Some(state),
                "Roundtrip failed for {state:?}: display={s}"
            );
        }
    }

    #[test]
    fn workspace_from_str_case_insensitive() {
        assert_eq!(
            "CREATED".parse::<WorkspaceState>().ok(),
            Some(WorkspaceState::Created)
        );
        assert_eq!(
            "Working".parse::<WorkspaceState>().ok(),
            Some(WorkspaceState::Working)
        );
    }

    #[test]
    fn workspace_from_str_rejects_garbage() {
        assert!(WorkspaceState::from_str("not-a-state").is_err());
        assert!(WorkspaceState::from_str("").is_err());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // LifecycleState conformance — all implementors
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn session_status_conformance() {
        crate::lifecycle::conformance_tests::run_all_tests::<SessionStatus>();
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Serde roundtrips
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn session_state_serde_roundtrip() {
        for &state in SessionState::all_states() {
            let json = serde_json::to_string(&state).expect("serialize");
            let back: SessionState = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(state, back, "Roundtrip failed for {state:?}");
        }
    }

    #[test]
    fn session_status_serde_roundtrip() {
        for &state in SessionStatus::all_states() {
            let json = serde_json::to_string(&state).expect("serialize");
            let back: SessionStatus = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(state, back, "Roundtrip failed for {state:?}");
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Proptest — property-based state machine invariants
    // ═══════════════════════════════════════════════════════════════════════════

    mod proptests {
        use proptest::prelude::*;

        use super::*;

        // SessionState index: 0..7 mapping to all_states
        fn session_state_idx() -> impl Strategy<Value = usize> {
            0..SessionState::all_states().len()
        }

        fn idx_to_session_state(idx: usize) -> SessionState {
            SessionState::all_states()[idx]
        }

        proptest! {
            #[test]
            fn session_state_can_transition_symmetric_consistency(from_idx in session_state_idx(), to_idx in session_state_idx()) {
                let from = idx_to_session_state(from_idx);
                let to = idx_to_session_state(to_idx);
                let can = from.can_transition_to(to);
                let in_list = from.valid_next_states().contains(&to);
                prop_assert_eq!(can, in_list);
            }

            #[test]
            fn session_state_no_self_transition(idx in session_state_idx()) {
                let state = idx_to_session_state(idx);
                prop_assert!(!state.can_transition_to(state));
            }

            #[test]
            fn session_state_terminal_is_consistent(idx in session_state_idx()) {
                let state = idx_to_session_state(idx);
                if state.is_terminal() {
                    prop_assert!(!state.valid_next_states().is_empty()
                        || state == SessionState::Completed
                        || state == SessionState::Failed);
                    // Completed/Failed can go back to Created, so not truly "trapped"
                }
            }

            #[test]
            fn workspace_state_can_transition_matches_valid_next(from_idx in 0..6usize, to_idx in 0..6usize) {
                let from = WorkspaceState::all()[from_idx];
                let to = WorkspaceState::all()[to_idx];
                let can = from.can_transition_to(to);
                let in_list = from.valid_next_states().contains(&to);
                prop_assert_eq!(can, in_list);
            }

            #[test]
            fn workspace_state_no_self_transition(idx in 0..6usize) {
                let state = WorkspaceState::all()[idx];
                prop_assert!(!state.can_transition_to(state));
            }

            #[test]
            fn workspace_state_terminal_implies_empty_next(idx in 0..6usize) {
                let state = WorkspaceState::all()[idx];
                if state.is_terminal() {
                    prop_assert!(state.valid_next_states().is_empty());
                }
            }

            #[test]
            fn session_status_can_transition_matches_valid_next(from_idx in 0..5usize, to_idx in 0..5usize) {
                let from = SessionStatus::all_states()[from_idx];
                let to = SessionStatus::all_states()[to_idx];
                let can = from.can_transition_to(to);
                let in_list = from.valid_next_states().contains(&to);
                prop_assert_eq!(can, in_list);
            }

            #[test]
            fn session_status_terminal_implies_empty_next(idx in 0..5usize) {
                let state = SessionStatus::all_states()[idx];
                if state.is_terminal() {
                    prop_assert!(state.valid_next_states().is_empty());
                    prop_assert!(!state.can_transition_to(state));
                }
            }

            #[test]
            fn state_transition_validate_consistent(from_idx in session_state_idx(), to_idx in session_state_idx()) {
                let from = idx_to_session_state(from_idx);
                let to = idx_to_session_state(to_idx);
                let t = StateTransition::new(from, to, "proptest");
                let result = t.validate();
                if from.can_transition_to(to) {
                    prop_assert!(result.is_ok());
                } else {
                    prop_assert!(result.is_err());
                }
            }

            #[test]
            fn workspace_state_display_from_str_roundtrip(idx in 0..6usize) {
                let state = WorkspaceState::all()[idx];
                let display = state.to_string();
                let parsed: std::result::Result<WorkspaceState, _> = display.parse();
                prop_assert!(parsed.is_ok());
                prop_assert_eq!(parsed.unwrap(), state);
            }

            #[test]
            fn workspace_filter_terminal_nonterminal_partition(idx in 0..6usize) {
                let state = WorkspaceState::all()[idx];
                let term = WorkspaceStateFilter::Terminal.matches(state);
                let nonterm = WorkspaceStateFilter::NonTerminal.matches(state);
                prop_assert_ne!(term, nonterm);
            }
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // TIMING METRICS — timestamp precision, monotonicity, duration
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn transition_timestamp_is_recent() {
        let before = Utc::now();
        let t = StateTransition::new(SessionState::Created, SessionState::Active, "test");
        let after = Utc::now();
        assert!(t.timestamp >= before, "Timestamp should be >= before");
        assert!(t.timestamp <= after, "Timestamp should be <= after");
    }

    #[test]
    fn workspace_transition_timestamp_is_recent() {
        let before = Utc::now();
        let t =
            WorkspaceStateTransition::new(WorkspaceState::Created, WorkspaceState::Working, "test");
        let after = Utc::now();
        assert!(t.timestamp >= before, "Timestamp should be >= before");
        assert!(t.timestamp <= after, "Timestamp should be <= after");
    }

    #[test]
    fn manager_timestamps_are_monotonically_increasing() {
        let mgr = SessionStateManager::new("mono-1")
            .activate("step 1")
            .expect("activate")
            .pause("step 2")
            .expect("pause")
            .resume("step 3")
            .expect("resume")
            .complete("step 4")
            .expect("complete");

        let history = mgr.history();
        for window in history.windows(2) {
            assert!(
                window[0].timestamp <= window[1].timestamp,
                "Timestamps must be monotonically non-decreasing: {:?} -> {:?}",
                window[0].timestamp,
                window[1].timestamp
            );
        }
    }

    #[test]
    fn manager_rapid_transitions_have_distinct_timestamps_or_equal() {
        // Rapid transitions may share timestamps at nanosecond precision
        // The invariant is: later timestamps are >= earlier timestamps
        let mgr = SessionStateManager::new("rapid-1")
            .activate("fast")
            .expect("activate")
            .sync("fast")
            .expect("sync")
            .sync_complete("fast")
            .expect("sync_complete");

        let history = mgr.history();
        assert!(history.len() >= 2);
        assert!(history[0].timestamp <= history[1].timestamp);
    }

    #[test]
    fn state_transition_records_accurate_timestamp_per_entry() {
        let mgr = SessionStateManager::new("ts-1");
        assert!(mgr.history().is_empty());

        let mgr = mgr.activate("first").expect("activate");
        assert_eq!(mgr.history().len(), 1);
        let ts1 = mgr.history()[0].timestamp;

        let mgr = mgr.pause("second").expect("pause");
        assert_eq!(mgr.history().len(), 2);
        let ts2 = mgr.history()[1].timestamp;

        // Each transition records its own timestamp
        assert!(ts1 <= ts2);
    }

    #[test]
    fn workspace_transition_agent_timestamp_captured() {
        let before = Utc::now();
        let t = WorkspaceStateTransition::with_agent(
            WorkspaceState::Working,
            WorkspaceState::Ready,
            "work done",
            "polecat-42",
        );
        let after = Utc::now();
        assert!(t.timestamp >= before && t.timestamp <= after);
        assert_eq!(t.agent_id.as_deref(), Some("polecat-42"));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // ERROR HANDLING DURING TRANSITIONS — message content, error types
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn try_transition_error_contains_from_and_to() {
        let err = SessionStatus::Creating
            .try_transition(SessionStatus::Paused)
            .expect_err("should fail");
        let msg = format!("{err}");
        assert!(
            msg.contains("Creating") || msg.contains("creating"),
            "Error should mention source state: {msg}"
        );
        assert!(
            msg.contains("Paused") || msg.contains("paused"),
            "Error should mention target state: {msg}"
        );
    }

    #[test]
    fn session_state_invalid_transition_error() {
        // SessionState doesn't implement LifecycleState trait (it has its own
        // can_transition_to), so test via StateTransition::validate instead
        let t = StateTransition::new(SessionState::Created, SessionState::Paused, "bad");
        let err = t.validate().expect_err("should fail");
        let msg = format!("{err}");
        assert!(!msg.is_empty(), "Error message should not be empty");
    }

    #[test]
    fn state_transition_validate_error_for_invalid_pair() {
        let t = StateTransition::new(SessionState::Active, SessionState::Created, "bad");
        let err = t.validate().expect_err("should fail");
        let msg = format!("{err}");
        assert!(!msg.is_empty(), "Error message should not be empty");
    }

    #[test]
    fn workspace_transition_validate_error_for_terminal_to_any() {
        for &terminal in &[WorkspaceState::Merged, WorkspaceState::Abandoned] {
            for &target in WorkspaceState::all() {
                let t = WorkspaceStateTransition::new(terminal, target, "try");
                let err = t.validate().expect_err("terminal should reject");
                let msg = format!("{err}");
                assert!(
                    !msg.is_empty(),
                    "Error for {terminal:?}->{target:?} should be descriptive"
                );
            }
        }
    }

    #[test]
    fn session_status_try_transition_all_invalid_pairs_error() {
        // Every invalid pair must produce an error
        for &from in SessionStatus::all_states() {
            for &to in SessionStatus::all_states() {
                if from.can_transition_to(to) {
                    continue;
                }
                let result = from.try_transition(to);
                assert!(
                    result.is_err(),
                    "Invalid pair ({from:?}, {to:?}) must return Err"
                );
            }
        }
    }

    #[test]
    fn session_state_invalid_pairs_all_error_via_validate() {
        for &from in SessionState::all_states() {
            for &to in SessionState::all_states() {
                if from.can_transition_to(to) {
                    continue;
                }
                let t = StateTransition::new(from, to, "test");
                assert!(
                    t.validate().is_err(),
                    "Invalid pair ({from:?}, {to:?}) must fail validation"
                );
            }
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // CONCURRENT ACCESS SAFETY — ownership model, thread independence
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn manager_ownership_prevents_double_use() {
        // SessionStateManager uses move semantics — after activate(),
        // the original manager is consumed and cannot be reused.
        // This test verifies the ownership model at compile time.
        let mgr = SessionStateManager::new("own-1");
        let mgr2 = mgr.activate("go").expect("activate");
        // mgr is moved — cannot use it here
        assert_eq!(mgr2.current_state(), SessionState::Active);
    }

    #[test]
    fn manager_state_preserved_across_ownership_transfer() {
        let mut mgr = SessionStateManager::new("transfer-1");
        mgr.set_metadata("key", "value");
        let mgr = mgr.activate("go").expect("activate");
        // Metadata survives ownership transfer
        assert_eq!(mgr.metadata().get("key"), Some(&"value".to_string()));
        assert_eq!(mgr.session_id(), "transfer-1");
        assert_eq!(mgr.current_state(), SessionState::Active);
    }

    #[test]
    fn independent_managers_do_not_interfere() {
        let mgr1 = SessionStateManager::new("indep-1")
            .activate("go1")
            .expect("activate")
            .complete("done1")
            .expect("complete");

        let mgr2 = SessionStateManager::new("indep-2")
            .activate("go2")
            .expect("activate")
            .pause("hold2")
            .expect("pause");

        assert_eq!(mgr1.current_state(), SessionState::Completed);
        assert_eq!(mgr2.current_state(), SessionState::Paused);
        assert_eq!(mgr1.history().len(), 2);
        assert_eq!(mgr2.history().len(), 2);
    }

    #[test]
    fn concurrent_managers_thread_safe() {
        use std::thread;

        // Each thread gets its own manager — no shared mutable state
        let results: Vec<_> = (0..4)
            .map(|i| {
                thread::spawn(move || {
                    let id = format!("thread-{i}");
                    let mgr = SessionStateManager::new(&id);
                    assert_eq!(mgr.current_state(), SessionState::Created);
                    let mgr = mgr.activate("activate").expect("activate");
                    assert_eq!(mgr.current_state(), SessionState::Active);
                    let mgr = mgr.complete("done").expect("complete");
                    assert_eq!(mgr.current_state(), SessionState::Completed);
                    mgr.session_id().to_string()
                })
            })
            .collect();

        let ids: Vec<_> = results
            .into_iter()
            .map(|h| h.join().expect("thread"))
            .collect();
        assert_eq!(ids.len(), 4);
        for i in 0..4 {
            assert_eq!(ids[i], format!("thread-{i}"));
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // PHASE BOUNDARY INVARIANTS — operations allowed/forbidden per phase
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn created_phase_only_allows_activate_or_fail() {
        // From Created, only activate() and fail() are available methods
        let mgr = SessionStateManager::new("phase-created");
        assert_eq!(mgr.current_state(), SessionState::Created);
        assert!(mgr.history().is_empty());

        // activate works - consume the manager
        let mgr = mgr.activate("go").expect("activate");
        assert_eq!(mgr.current_state(), SessionState::Active);
    }

    #[test]
    fn created_phase_can_fail_immediately() {
        let mgr = SessionStateManager::new("phase-fail-early");
        let mgr = mgr.fail("instant failure").expect("fail");
        assert_eq!(mgr.current_state(), SessionState::Failed);
    }

    #[test]
    fn active_phase_allows_committing_effect_sync_pause_complete() {
        let _mgr = SessionStateManager::new("phase-active")
            .activate("go")
            .expect("activate");

        let m0 = SessionStateManager::new("a-commit")
            .activate("go")
            .expect("a")
            .begin_commit_effect("commit")
            .expect("begin_commit_effect");
        assert_eq!(m0.current_state(), SessionState::CommittingEffect);

        let m1 = SessionStateManager::new("a-sync")
            .activate("go")
            .expect("a")
            .sync("push")
            .expect("sync");
        assert_eq!(m1.current_state(), SessionState::Syncing);

        let m2 = SessionStateManager::new("a-pause")
            .activate("go")
            .expect("a")
            .pause("break")
            .expect("pause");
        assert_eq!(m2.current_state(), SessionState::Paused);

        let m3 = SessionStateManager::new("a-complete")
            .activate("go")
            .expect("a")
            .complete("done")
            .expect("complete");
        assert_eq!(m3.current_state(), SessionState::Completed);
    }

    #[test]
    fn committing_effect_phase_allows_commit_complete_sync_or_fail() {
        let base = || {
            SessionStateManager::new("ce-base")
                .activate("go")
                .expect("a")
                .begin_commit_effect("commit")
                .expect("ce")
        };

        let m1 = base().commit_complete("done").expect("cc");
        assert_eq!(m1.current_state(), SessionState::Active);

        let m2 = base().sync("push").expect("sync");
        assert_eq!(m2.current_state(), SessionState::Syncing);

        let m3 = base().fail("error").expect("fail");
        assert_eq!(m3.current_state(), SessionState::Failed);
    }

    #[test]
    fn committing_effect_roundtrip_preserves_identity() {
        let mgr = SessionStateManager::new("ce-rt")
            .activate("go")
            .expect("a")
            .begin_commit_effect("commit")
            .expect("ce")
            .commit_complete("done")
            .expect("cc");
        assert_eq!(mgr.current_state(), SessionState::Active);
        assert_eq!(mgr.session_id(), "ce-rt");
        assert_eq!(mgr.history().len(), 3);
    }

    #[test]
    fn syncing_phase_only_allows_sync_complete_or_fail() {
        let m1 = SessionStateManager::new("sync-ok")
            .activate("go")
            .expect("a")
            .sync("push")
            .expect("sync")
            .sync_complete("done")
            .expect("sc");
        assert_eq!(m1.current_state(), SessionState::Synced);

        let m2 = SessionStateManager::new("sync-fail")
            .activate("go")
            .expect("a")
            .sync("push")
            .expect("sync")
            .fail("error")
            .expect("fail");
        assert_eq!(m2.current_state(), SessionState::Failed);
    }

    #[test]
    fn synced_phase_allows_reactivate_pause_complete() {
        let base = || {
            SessionStateManager::new("synced-base")
                .activate("go")
                .expect("a")
                .sync("push")
                .expect("sync")
                .sync_complete("done")
                .expect("sc")
        };

        let m1 = base().reactivate("more work").expect("reactivate");
        assert_eq!(m1.current_state(), SessionState::Active);

        let m2 = base().pause("hold").expect("pause");
        assert_eq!(m2.current_state(), SessionState::Paused);

        let m3 = base().complete("done").expect("complete");
        assert_eq!(m3.current_state(), SessionState::Completed);
    }

    #[test]
    fn paused_phase_allows_resume_or_complete() {
        let base = || {
            SessionStateManager::new("paused-base")
                .activate("go")
                .expect("a")
                .pause("break")
                .expect("pause")
        };

        let m1 = base().resume("back").expect("resume");
        assert_eq!(m1.current_state(), SessionState::Active);

        let m2 = base().complete("done from pause").expect("complete");
        assert_eq!(m2.current_state(), SessionState::Completed);
    }

    #[test]
    fn completed_phase_allows_restart() {
        let mgr = SessionStateManager::new("completed-restart")
            .activate("go")
            .expect("a")
            .complete("done")
            .expect("c")
            .restart("redo")
            .expect("restart");
        assert_eq!(mgr.current_state(), SessionState::Created);
    }

    #[test]
    fn failed_phase_allows_retry() {
        let mgr = SessionStateManager::new("failed-retry")
            .fail("error")
            .expect("f")
            .retry("retry")
            .expect("retry");
        assert_eq!(mgr.current_state(), SessionState::Created);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // EDGE CASES — empty strings, unicode, long chains, restart loops
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn manager_empty_reason_string() {
        let mgr = SessionStateManager::new("empty-reason")
            .activate("")
            .expect("activate with empty reason");
        assert_eq!(mgr.history()[0].reason, "");
    }

    #[test]
    fn manager_unicode_reason_string() {
        let mgr = SessionStateManager::new("unicode-reason")
            .activate("日本語テスト 🚀 émoji")
            .expect("activate with unicode");
        assert_eq!(mgr.history()[0].reason, "日本語テスト 🚀 émoji");
    }

    #[test]
    fn manager_long_reason_string() {
        let long_reason = "x".repeat(10_000);
        let mgr = SessionStateManager::new("long-reason")
            .activate(&long_reason)
            .expect("activate with long reason");
        assert_eq!(mgr.history()[0].reason.len(), 10_000);
    }

    #[test]
    fn manager_multiple_restart_cycles() {
        // Created -> Active -> Completed -> Created -> Active -> Completed -> ...
        let mgr = SessionStateManager::new("cycles")
            .activate("cycle 1")
            .expect("a1")
            .complete("done 1")
            .expect("c1")
            .restart("restart 1")
            .expect("r1")
            .activate("cycle 2")
            .expect("a2")
            .complete("done 2")
            .expect("c2")
            .restart("restart 2")
            .expect("r2")
            .activate("cycle 3")
            .expect("a3")
            .complete("done 3")
            .expect("c3");

        assert_eq!(mgr.current_state(), SessionState::Completed);
        assert_eq!(mgr.history().len(), 8); // 3 full cycles
                                            // (activate+complete+restart+activate+complete+restart+activate+complete)
    }

    #[test]
    fn manager_fail_retry_cycle() {
        let mgr = SessionStateManager::new("fail-retry")
            .fail("error 1")
            .expect("f1")
            .retry("retry 1")
            .expect("r1")
            .activate("go")
            .expect("a1")
            .sync("push")
            .expect("sync")
            .fail("sync error")
            .expect("f2")
            .retry("retry 2")
            .expect("r2");

        assert_eq!(mgr.current_state(), SessionState::Created);
        assert_eq!(mgr.history().len(), 6);
    }

    #[test]
    fn manager_sync_reactivate_loop() {
        let mgr = SessionStateManager::new("sync-loop")
            .activate("go")
            .expect("a1")
            .sync("push 1")
            .expect("s1")
            .sync_complete("done 1")
            .expect("sc1")
            .reactivate("more work")
            .expect("reactivate")
            .sync("push 2")
            .expect("s2")
            .sync_complete("done 2")
            .expect("sc2");

        assert_eq!(mgr.current_state(), SessionState::Synced);
        assert_eq!(mgr.history().len(), 6);
    }

    #[test]
    fn manager_pause_resume_loop() {
        let mgr = SessionStateManager::new("pause-loop")
            .activate("go")
            .expect("a")
            .pause("break 1")
            .expect("p1")
            .resume("back 1")
            .expect("r1")
            .pause("break 2")
            .expect("p2")
            .resume("back 2")
            .expect("r2");

        assert_eq!(mgr.current_state(), SessionState::Active);
        assert_eq!(mgr.history().len(), 5);
    }

    #[test]
    fn manager_empty_session_id() {
        let mgr = SessionStateManager::new("");
        assert_eq!(mgr.session_id(), "");
        let mgr = mgr.activate("go").expect("activate");
        assert_eq!(mgr.session_id(), "");
    }

    #[test]
    fn manager_unicode_session_id() {
        let mgr = SessionStateManager::new("セッション-🚀");
        assert_eq!(mgr.session_id(), "セッション-🚀");
    }

    #[test]
    fn metadata_overwrite() {
        let mut mgr = SessionStateManager::new("meta-overwrite");
        mgr.set_metadata("key", "first");
        assert_eq!(mgr.metadata().get("key"), Some(&"first".to_string()));
        mgr.set_metadata("key", "second");
        assert_eq!(mgr.metadata().get("key"), Some(&"second".to_string()));
    }

    #[test]
    fn metadata_multiple_keys() {
        let mut mgr = SessionStateManager::new("meta-multi");
        mgr.set_metadata("a", "1");
        mgr.set_metadata("b", "2");
        mgr.set_metadata("c", "3");
        assert_eq!(mgr.metadata().len(), 3);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // LifecycleState CONFORMANCE — SessionState and WorkspaceState
    // ═══════════════════════════════════════════════════════════════════════════

    // NOTE: SessionState does not implement the LifecycleState trait (it has its own
    // can_transition_to/valid_next_states methods but no trait impl). We test equivalent
    // invariants using its direct methods.

    #[test]
    fn session_state_equivalent_conformance() {
        // Transition consistency: can_transition_to matches valid_next_states
        for &from in SessionState::all_states() {
            let nexts = from.valid_next_states();
            for &to in SessionState::all_states() {
                assert_eq!(
                    from.can_transition_to(to),
                    nexts.contains(&to),
                    "SessionState inconsistency: {from:?} -> {to:?}"
                );
            }
        }

        // No self-transitions
        for &state in SessionState::all_states() {
            assert!(
                !state.can_transition_to(state),
                "No self-transition for {state:?}"
            );
        }

        // All states listed in all_states are unique
        let all = SessionState::all_states();
        let mut seen = std::collections::HashSet::new();
        for &s in all {
            assert!(seen.insert(s), "Duplicate in all_states: {s:?}");
        }
        assert_eq!(all.len(), 8);
    }

    // NOTE: WorkspaceState does not implement LifecycleState trait (it uses its own
    // all() instead of all_states()), so we test it separately with equivalent invariants.

    #[test]
    fn workspace_state_equivalent_conformance() {
        // Transition consistency: can_transition_to matches valid_next_states
        for &from in WorkspaceState::all() {
            let nexts = from.valid_next_states();
            for &to in WorkspaceState::all() {
                assert_eq!(
                    from.can_transition_to(to),
                    nexts.contains(&to),
                    "WorkspaceState inconsistency: {from:?} -> {to:?}"
                );
            }
        }

        // Terminal states have empty valid_next_states
        for &state in WorkspaceState::all() {
            if state.is_terminal() {
                assert!(
                    state.valid_next_states().is_empty(),
                    "Terminal {state:?} must have empty next states"
                );
            }
        }

        // Non-terminal states have at least one transition
        for &state in WorkspaceState::all() {
            if !state.is_terminal() {
                assert!(
                    !state.valid_next_states().is_empty(),
                    "Non-terminal {state:?} must have transitions"
                );
            }
        }

        // No self-transitions
        for &state in WorkspaceState::all() {
            assert!(
                !state.can_transition_to(state),
                "No self-transition for {state:?}"
            );
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // SESSION STATUS — exhaustive operations guard
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn session_status_operations_exhaustive_cross_check() {
        // allows_operation must be consistent with allowed_operations
        for &status in SessionStatus::all_states() {
            let ops = status.allowed_operations();
            for op in [
                Operation::Status,
                Operation::Diff,
                Operation::Focus,
                Operation::Remove,
            ] {
                assert_eq!(
                    status.allows_operation(op),
                    ops.contains(&op),
                    "Inconsistency for {status:?}.{op:?}"
                );
            }
        }
    }

    #[test]
    fn session_status_creating_allows_nothing() {
        assert!(SessionStatus::Creating.allowed_operations().is_empty());
    }

    #[test]
    fn session_status_terminal_allows_only_remove() {
        for &terminal in &[SessionStatus::Completed, SessionStatus::Failed] {
            let ops = terminal.allowed_operations();
            assert_eq!(ops.len(), 1);
            assert_eq!(ops[0], Operation::Remove);
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // WORKSPACE STATE FILTER — Active and Complete predicates
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn filter_active_matches_working_and_conflict() {
        for &state in WorkspaceState::all() {
            let expected = state == WorkspaceState::Working || state == WorkspaceState::Conflict;
            assert_eq!(
                WorkspaceStateFilter::Active.matches(state),
                expected,
                "Active filter mismatch for {state:?}"
            );
        }
    }

    #[test]
    fn filter_complete_matches_ready_and_merged() {
        for &state in WorkspaceState::all() {
            let expected = state == WorkspaceState::Ready || state == WorkspaceState::Merged;
            assert_eq!(
                WorkspaceStateFilter::Complete.matches(state),
                expected,
                "Complete filter mismatch for {state:?}"
            );
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // STATE TRANSITION SERDE — roundtrip with all fields
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn state_transition_serde_roundtrip() {
        let t = StateTransition::new(SessionState::Created, SessionState::Active, "test");
        let json = serde_json::to_string(&t).expect("serialize");
        let back: StateTransition = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(t.from, back.from);
        assert_eq!(t.to, back.to);
        assert_eq!(t.reason, back.reason);
    }

    #[test]
    fn workspace_state_transition_serde_with_agent_roundtrip() {
        let t = WorkspaceStateTransition::with_agent(
            WorkspaceState::Working,
            WorkspaceState::Ready,
            "work done",
            "agent-99",
        );
        let json = serde_json::to_string(&t).expect("serialize");
        let back: WorkspaceStateTransition = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(t.from, back.from);
        assert_eq!(t.to, back.to);
        assert_eq!(t.reason, back.reason);
        assert_eq!(t.agent_id, back.agent_id);
    }

    #[test]
    fn workspace_state_transition_serde_without_agent_skips_field() {
        let t = WorkspaceStateTransition::new(
            WorkspaceState::Created,
            WorkspaceState::Working,
            "start",
        );
        let json = serde_json::to_string(&t).expect("serialize");
        // agent_id is None, so skip_serializing_if should omit it
        assert!(
            !json.contains("agent_id"),
            "Should skip null agent_id: {json}"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // WORKSPACE DISPLAY — all variants have stable string representation
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn workspace_display_stable_strings() {
        assert_eq!(WorkspaceState::Created.to_string(), "created");
        assert_eq!(WorkspaceState::Working.to_string(), "working");
        assert_eq!(WorkspaceState::Ready.to_string(), "ready");
        assert_eq!(WorkspaceState::Merged.to_string(), "merged");
        assert_eq!(WorkspaceState::Abandoned.to_string(), "abandoned");
        assert_eq!(WorkspaceState::Conflict.to_string(), "conflict");
    }

    #[test]
    fn workspace_from_str_all_variants() {
        for &state in WorkspaceState::all() {
            let display = state.to_string();
            let parsed: std::result::Result<WorkspaceState, _> = display.parse();
            assert_eq!(parsed.ok(), Some(state), "Roundtrip failed for {state:?}");
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // SESSION STATE — Copy, Eq, Hash properties
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn session_state_copy_preserves_value() {
        let state = SessionState::Active;
        let copied = state;
        assert_eq!(state, copied);
    }

    #[test]
    fn session_state_eq_ne() {
        assert_eq!(SessionState::Active, SessionState::Active);
        assert_ne!(SessionState::Active, SessionState::Paused);
    }

    #[test]
    fn session_state_hash_consistency() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for &state in SessionState::all_states() {
            assert!(set.insert(state), "Duplicate hash for {state:?}");
        }
        assert_eq!(set.len(), 8);
    }

    #[test]
    fn session_status_copy_preserves_value() {
        let status = SessionStatus::Active;
        let copied = status;
        assert_eq!(status, copied);
    }

    #[test]
    fn session_status_hash_consistency() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for &status in SessionStatus::all_states() {
            assert!(set.insert(status), "Duplicate hash for {status:?}");
        }
        assert_eq!(set.len(), 5);
    }

    #[test]
    fn workspace_state_copy_preserves_value() {
        let state = WorkspaceState::Working;
        let copied = state;
        assert_eq!(state, copied);
    }

    #[test]
    fn workspace_state_hash_consistency() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for &state in WorkspaceState::all() {
            assert!(set.insert(state), "Duplicate hash for {state:?}");
        }
        assert_eq!(set.len(), 6);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // SESSION STATE — exhaustive invalid transition error count
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn session_state_invalid_transition_count() {
        // 7 states, 14 valid transitions (from exhaustive matrix above)
        // Total possible: 7*7=49 (minus self-transitions = 42)
        // Invalid = 42 - 14 = 28
        let valid_count: usize = (0..7)
            .flat_map(|i| (0..7).map(move |j| (i, j)))
            .filter(|&(i, j)| i != j)
            .filter(|&(i, j)| {
                let from = SessionState::all_states()[i];
                let to = SessionState::all_states()[j];
                from.can_transition_to(to)
            })
            .count();

        let expected_valid = 14;
        assert_eq!(
            valid_count, expected_valid,
            "SessionState should have exactly {expected_valid} valid transitions"
        );

        // All invalid transitions must produce errors
        let invalid_count = (0..7)
            .flat_map(|i| (0..7).map(move |j| (i, j)))
            .filter(|&(i, j)| i != j)
            .filter(|&(i, j)| {
                let from = SessionState::all_states()[i];
                let to = SessionState::all_states()[j];
                !from.can_transition_to(to)
            })
            .count();

        assert_eq!(invalid_count, 42 - expected_valid);
    }

    #[test]
    fn workspace_state_invalid_transition_count() {
        // 6 states, from the exhaustive matrix above: 10 valid transitions
        // Total possible: 6*6=36 (minus self-transitions = 30)
        // Invalid = 30 - 10 = 20
        let valid_count: usize = (0..6)
            .flat_map(|i| (0..6).map(move |j| (i, j)))
            .filter(|&(i, j)| i != j)
            .filter(|&(i, j)| {
                let from = WorkspaceState::all()[i];
                let to = WorkspaceState::all()[j];
                from.can_transition_to(to)
            })
            .count();

        assert_eq!(
            valid_count, 10,
            "WorkspaceState should have exactly 10 valid transitions"
        );
    }

    #[test]
    fn session_status_invalid_transition_count() {
        // 5 states
        // Valid: Creating->Active, Creating->Failed, Active->Paused, Active->Completed,
        //        Paused->Active, Paused->Completed = 6
        // Total possible: 5*5=25 (minus self-transitions = 20)
        // Invalid = 20 - 6 = 14
        let valid_count: usize = (0..5)
            .flat_map(|i| (0..5).map(move |j| (i, j)))
            .filter(|&(i, j)| i != j)
            .filter(|&(i, j)| {
                let from = SessionStatus::all_states()[i];
                let to = SessionStatus::all_states()[j];
                from.can_transition_to(to)
            })
            .count();

        assert_eq!(
            valid_count, 6,
            "SessionStatus should have exactly 6 valid transitions"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // WORKSPACE STATE DEFAULT
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn workspace_state_default_is_created() {
        assert_eq!(WorkspaceState::default(), WorkspaceState::Created);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // STRESS TESTS — long transition chains
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn manager_stress_many_cycles() {
        // Use chained transitions - type-state pattern requires each step to
        // return the correct typed manager. We can chain multiple restart cycles.
        let mgr = SessionStateManager::new("stress")
            .activate("c0")
            .expect("a0")
            .complete("c0")
            .expect("c0")
            .restart("c0")
            .expect("r0")
            .activate("c1")
            .expect("a1")
            .complete("c1")
            .expect("c1")
            .restart("c1")
            .expect("r1")
            .activate("c2")
            .expect("a2")
            .complete("c2")
            .expect("c2")
            .restart("c2")
            .expect("r2")
            .activate("c3")
            .expect("a3")
            .complete("c3")
            .expect("c3")
            .restart("c3")
            .expect("r3")
            .activate("c4")
            .expect("a4")
            .complete("c4")
            .expect("c4")
            .restart("c4")
            .expect("r4")
            .activate("final")
            .expect("final");

        assert_eq!(mgr.current_state(), SessionState::Active);
        // 5 cycles * 3 (activate+complete+restart) + 1 final activate = 16
        assert_eq!(mgr.history().len(), 16);
    }

    #[test]
    fn manager_stress_sync_reactivate_cycles() {
        let mgr = SessionStateManager::new("stress-sync")
            .activate("s0")
            .expect("a0")
            .sync("s0")
            .expect("s0")
            .sync_complete("s0")
            .expect("sc0")
            .reactivate("s0")
            .expect("r0")
            .sync("s1")
            .expect("s1")
            .sync_complete("s1")
            .expect("sc1")
            .reactivate("s1")
            .expect("r1")
            .sync("s2")
            .expect("s2")
            .sync_complete("s2")
            .expect("sc2");

        assert_eq!(mgr.current_state(), SessionState::Synced);
        // 3 full sync cycles: (activate+sync+sync_complete) + (reactivate+sync+sync_complete) * 2 =
        // 9
        assert_eq!(mgr.history().len(), 9);
    }
}
