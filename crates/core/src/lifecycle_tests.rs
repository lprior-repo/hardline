//! Exhaustive tests for lifecycle state management, phase transitions,
//! transition guards, history tracking, and state machine invariants.
//!
//! Covers: SessionState, SessionStateManager, StateTransition,
//! WorkspaceState, WorkspaceStateTransition, WorkspaceStateFilter,
//! LifecycleState trait conformance, and property-based tests.

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::lifecycle::LifecycleState;
    use crate::session_state::{SessionState, SessionStateManager, StateTransition};
    use crate::type_session_status::{Operation, SessionStatus};
    use crate::workspace_state::{WorkspaceState, WorkspaceStateFilter, WorkspaceStateTransition};

    // ═══════════════════════════════════════════════════════════════════════════
    // SessionState — exhaustive transition matrix
    // ═══════════════════════════════════════════════════════════════════════════

    /// SessionState has 7 variants. Test every (from, to) pair exhaustively.
    #[test]
    fn session_state_exhaustive_transition_matrix() {
        let valid: Vec<(SessionState, SessionState)> = vec![
            (SessionState::Created, SessionState::Active),
            (SessionState::Created, SessionState::Failed),
            (SessionState::Active, SessionState::Syncing),
            (SessionState::Active, SessionState::Paused),
            (SessionState::Active, SessionState::Completed),
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
        assert_eq!(all.len(), 7);
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
        let mgr = SessionStateManager::new("sess-2").activate("go").expect("activate");
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
        let mgr = SessionStateManager::new("sess-6").fail("startup error").expect("fail");
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
            (SessionState::Active, SessionState::Syncing),
            (SessionState::Active, SessionState::Paused),
            (SessionState::Active, SessionState::Completed),
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
                assert!(t.validate().is_err(), "Should be invalid: {from:?} -> {to:?}");
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
            SessionStatus::Creating.try_transition(SessionStatus::Active).ok(),
            Some(SessionStatus::Active)
        );
        assert_eq!(
            SessionStatus::Active.try_transition(SessionStatus::Paused).ok(),
            Some(SessionStatus::Paused)
        );
        assert_eq!(
            SessionStatus::Active.try_transition(SessionStatus::Completed).ok(),
            Some(SessionStatus::Completed)
        );
        assert_eq!(
            SessionStatus::Paused.try_transition(SessionStatus::Active).ok(),
            Some(SessionStatus::Active)
        );
    }

    #[test]
    fn try_transition_err_for_all_invalid_session_status() {
        assert!(SessionStatus::Creating.try_transition(SessionStatus::Paused).is_err());
        assert!(SessionStatus::Creating.try_transition(SessionStatus::Creating).is_err());
        assert!(SessionStatus::Completed.try_transition(SessionStatus::Active).is_err());
        assert!(SessionStatus::Failed.try_transition(SessionStatus::Active).is_err());
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
        for op in [Operation::Status, Operation::Diff, Operation::Focus, Operation::Remove] {
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
            assert!(!state.can_transition_to(state), "No self-transition for {state:?}");
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
            assert!(!state.valid_next_states().is_empty(), "{state:?} should have transitions");
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
        let t = WorkspaceStateTransition::new(
            WorkspaceState::Working,
            WorkspaceState::Ready,
            "ready",
        );
        assert!(t.agent_id.is_none());
    }

    #[test]
    fn workspace_terminal_to_any_fails() {
        for &terminal in &[WorkspaceState::Merged, WorkspaceState::Abandoned] {
            for &target in WorkspaceState::all() {
                let t = WorkspaceStateTransition::new(terminal, target, "try");
                assert!(t.validate().is_err(), "Terminal {terminal:?} -> {target:?} should fail");
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
                    assert!(!filter.matches(other), "{state:?} filter should not match {other:?}");
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
            assert_ne!(terminal, non_terminal, "{state:?}: terminal={terminal}, non_terminal={non_terminal}");
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
            assert_eq!(parsed.ok(), Some(state), "Roundtrip failed for {state:?}: display={s}");
        }
    }

    #[test]
    fn workspace_from_str_case_insensitive() {
        assert_eq!("CREATED".parse::<WorkspaceState>().ok(), Some(WorkspaceState::Created));
        assert_eq!("Working".parse::<WorkspaceState>().ok(), Some(WorkspaceState::Working));
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
        use super::*;
        use proptest::prelude::*;

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
}
