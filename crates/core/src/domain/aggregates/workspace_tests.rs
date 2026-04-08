//! Exhaustive tests for Workspace aggregate — state management, transitions,
//! creation, metadata, builder, validation, path operations, and proptests.
//!
//! Transition matrix (5 states × 5 targets = 25 pairs):
//!
//!   From      → To        Valid?   Method
//!   ─────────────────────────────────────────────
//!   Creating  → Ready      ✓      `mark_ready`
//!   Creating  → Removed    ✓      `mark_removed`
//!   Ready     → Active     ✓      `mark_active`
//!   Ready     → Cleaning   ✓      `start_cleaning`
//!   Ready     → Removed    ✓      `mark_removed`
//!   Active    → Cleaning   ✓      `start_cleaning`
//!   Active    → Removed    ✓      `mark_removed`
//!   Cleaning  → Removed    ✓      `mark_removed`
//!   All others            ✗

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use proptest::prelude::*;

    use crate::domain::aggregates::workspace::Workspace;
    use crate::domain::aggregates::workspace_builder::WorkspaceBuilder;
    use crate::domain::aggregates::workspace_error::WorkspaceError;
    use crate::domain::identifiers::WorkspaceName;
    use crate::domain::workspace::WorkspaceState;

    // ── Helpers ───────────────────────────────────────────────────────────

    fn name(s: &str) -> WorkspaceName {
        WorkspaceName::parse(s).expect("valid name")
    }

    /// Create a workspace in Creating state at /tmp.
    fn ws_creating() -> Workspace {
        Workspace::create(name("test-ws"), PathBuf::from("/tmp")).expect("create")
    }

    /// Create a workspace in Ready state.
    fn ws_ready() -> Workspace {
        ws_creating().mark_ready().expect("→ ready")
    }

    /// Create a workspace in Active state.
    fn ws_active() -> Workspace {
        ws_ready().mark_active().expect("→ active")
    }

    /// Create a workspace in Cleaning state.
    fn ws_cleaning() -> Workspace {
        ws_active().start_cleaning().expect("→ cleaning")
    }

    /// Create a workspace in Removed state.
    fn ws_removed() -> Workspace {
        ws_creating().mark_removed().expect("→ removed")
    }

    /// Build a workspace in the given state via reconstruct.
    fn ws_in_state(state: WorkspaceState) -> Workspace {
        Workspace::reconstruct(name("state-ws"), PathBuf::from("/tmp"), state)
            .expect("reconstruct")
    }

    // ========================================================================
    // 1. CREATION
    // ========================================================================

    #[test]
    fn create_with_valid_params_starts_in_creating() {
        let ws = ws_creating();
        assert!(ws.is_creating());
        assert_eq!(ws.name, name("test-ws"));
        assert_eq!(ws.path, PathBuf::from("/tmp"));
        assert_eq!(ws.state, WorkspaceState::Creating);
    }

    #[test]
    fn create_with_nonexistent_path_returns_path_not_found() {
        let result = Workspace::create(name("bad"), PathBuf::from("/no/such/path/ever"));
        assert!(matches!(result, Err(WorkspaceError::PathNotFound(_))));
    }

    #[test]
    fn create_with_various_valid_names() {
        for n in &["a", "my-workspace", "ws_01", "workspace.v2", &"x".repeat(255)] {
            let ws = Workspace::create(name(n), PathBuf::from("/tmp")).expect("valid");
            assert_eq!(ws.name.as_str(), *n);
        }
    }

    #[test]
    fn create_preserves_path_exactly() {
        let ws = Workspace::create(name("p"), PathBuf::from("/tmp")).expect("ok");
        assert_eq!(ws.path, PathBuf::from("/tmp"));
    }

    #[test]
    fn create_sets_state_to_creating() {
        assert_eq!(ws_creating().state, WorkspaceState::Creating);
    }

    #[test]
    fn create_error_contains_path() {
        let bad_path = PathBuf::from("/nonexistent_xyz_abc");
        let err = Workspace::create(name("e"), bad_path.clone()).unwrap_err();
        match err {
            WorkspaceError::PathNotFound(p) => assert_eq!(p, bad_path),
            other => panic!("expected PathNotFound, got {other:?}"),
        }
    }

    // ========================================================================
    // 2. RECONSTRUCTION
    // ========================================================================

    #[test]
    fn reconstruct_with_each_state() {
        for state in WorkspaceState::all() {
            let ws = Workspace::reconstruct(name("r"), PathBuf::from("/tmp"), state)
                .expect("reconstruct should work for existing path");
            assert_eq!(ws.state, state);
        }
    }

    #[test]
    fn reconstruct_with_nonexistent_path_fails() {
        let result = Workspace::reconstruct(
            name("r"),
            PathBuf::from("/no/such/path"),
            WorkspaceState::Ready,
        );
        assert!(matches!(result, Err(WorkspaceError::PathNotFound(_))));
    }

    #[test]
    fn reconstruct_preserves_name() {
        let ws =
            Workspace::reconstruct(name("my-name"), PathBuf::from("/tmp"), WorkspaceState::Active)
                .expect("ok");
        assert_eq!(ws.name.as_str(), "my-name");
    }

    #[test]
    fn reconstruct_preserves_path() {
        let ws = Workspace::reconstruct(
            name("p"),
            PathBuf::from("/tmp"),
            WorkspaceState::Cleaning,
        )
        .expect("ok");
        assert_eq!(ws.path, PathBuf::from("/tmp"));
    }

    // ========================================================================
    // 3. FULL TRANSITION MATRIX — 25 pairs (5 × 5)
    // ========================================================================

    #[test]
    fn all_valid_transitions_succeed() {
        // Creating → Ready
        let ws = ws_creating().mark_ready().expect("Creating → Ready");
        assert_eq!(ws.state, WorkspaceState::Ready);

        // Creating → Removed
        let ws = ws_creating().mark_removed().expect("Creating → Removed");
        assert_eq!(ws.state, WorkspaceState::Removed);

        // Ready → Active
        let ws = ws_ready().mark_active().expect("Ready → Active");
        assert_eq!(ws.state, WorkspaceState::Active);

        // Ready → Cleaning
        let ws = ws_ready().start_cleaning().expect("Ready → Cleaning");
        assert_eq!(ws.state, WorkspaceState::Cleaning);

        // Ready → Removed
        let ws = ws_ready().mark_removed().expect("Ready → Removed");
        assert_eq!(ws.state, WorkspaceState::Removed);

        // Active → Cleaning
        let ws = ws_active().start_cleaning().expect("Active → Cleaning");
        assert_eq!(ws.state, WorkspaceState::Cleaning);

        // Active → Removed
        let ws = ws_active().mark_removed().expect("Active → Removed");
        assert_eq!(ws.state, WorkspaceState::Removed);

        // Cleaning → Removed
        let ws = ws_cleaning().mark_removed().expect("Cleaning → Removed");
        assert_eq!(ws.state, WorkspaceState::Removed);
    }

    #[test]
    fn all_invalid_transitions_fail() {
        // Helper: try each method from each state. Any method that doesn't
        // match the valid transition matrix must return InvalidStateTransition.
        //
        // Methods: mark_ready (→Ready), mark_active (→Active),
        //          start_cleaning (→Cleaning), mark_removed (→Removed)
        // No public method targets Creating.

        struct MethodCall {
            label: &'static str,
            invoke: fn(&Workspace) -> Result<Workspace, WorkspaceError>,
        }
        let methods = [
            MethodCall { label: "mark_ready", invoke: Workspace::mark_ready as fn(&Workspace) -> Result<Workspace, WorkspaceError> },
            MethodCall { label: "mark_active", invoke: Workspace::mark_active as fn(&Workspace) -> Result<Workspace, WorkspaceError> },
            MethodCall { label: "start_cleaning", invoke: Workspace::start_cleaning as fn(&Workspace) -> Result<Workspace, WorkspaceError> },
            MethodCall { label: "mark_removed", invoke: Workspace::mark_removed as fn(&Workspace) -> Result<Workspace, WorkspaceError> },
        ];

        // Map method label to target state
        fn target_for(label: &str) -> WorkspaceState {
            match label {
                "mark_ready" => WorkspaceState::Ready,
                "mark_active" => WorkspaceState::Active,
                "start_cleaning" => WorkspaceState::Cleaning,
                "mark_removed" => WorkspaceState::Removed,
                _ => unreachable!(),
            }
        }

        for state in WorkspaceState::all() {
            let ws = ws_in_state(state);
            for method in &methods {
                let target = target_for(method.label);
                if state.can_transition_to(&target) {
                    // Valid — should succeed
                    let result = (method.invoke)(&ws);
                    assert!(
                        result.is_ok(),
                        "{method_label} from {state:?} should succeed",
                        method_label = method.label
                    );
                } else {
                    // Invalid — must fail
                    let result = (method.invoke)(&ws);
                    assert!(
                        matches!(result, Err(WorkspaceError::InvalidStateTransition { .. })),
                        "{method_label} from {state:?} (→ {target:?}) should fail",
                        method_label = method.label,
                        target = target
                    );
                }
            }
        }
    }

    #[test]
    fn invalid_transition_error_preserves_from_and_to() {
        let ws = ws_creating(); // Creating
        let err = ws.mark_active().unwrap_err(); // Creating → Active is invalid
        match err {
            WorkspaceError::InvalidStateTransition { from, to } => {
                assert_eq!(from, WorkspaceState::Creating);
                assert_eq!(to, WorkspaceState::Active);
            }
            other => panic!("expected InvalidStateTransition, got {other:?}"),
        }
    }

    #[test]
    fn self_transitions_all_invalid() {
        // mark_ready from Ready (self-loop)
        assert!(matches!(
            ws_ready().mark_ready(),
            Err(WorkspaceError::InvalidStateTransition { .. })
        ));
        // mark_active from Active (self-loop)
        assert!(matches!(
            ws_active().mark_active(),
            Err(WorkspaceError::InvalidStateTransition { .. })
        ));
        // start_cleaning from Cleaning (self-loop)
        assert!(matches!(
            ws_cleaning().start_cleaning(),
            Err(WorkspaceError::InvalidStateTransition { .. })
        ));
        // mark_removed from Removed (self-loop)
        assert!(matches!(
            ws_removed().mark_removed(),
            Err(WorkspaceError::InvalidStateTransition { .. })
        ));
        // No method transitions to Creating; verify all methods fail from Creating
        let creating = ws_creating();
        assert!(matches!(
            creating.mark_active(),
            Err(WorkspaceError::InvalidStateTransition { .. })
        ));
        assert!(matches!(
            creating.start_cleaning(),
            Err(WorkspaceError::InvalidStateTransition { .. })
        ));
    }

    #[test]
    fn terminal_state_rejects_all_transitions() {
        let removed = ws_removed();
        assert!(matches!(
            removed.mark_ready(),
            Err(WorkspaceError::InvalidStateTransition { .. })
        ));
        assert!(matches!(
            removed.mark_active(),
            Err(WorkspaceError::InvalidStateTransition { .. })
        ));
        assert!(matches!(
            removed.start_cleaning(),
            Err(WorkspaceError::InvalidStateTransition { .. })
        ));
        assert!(matches!(
            removed.mark_removed(),
            Err(WorkspaceError::InvalidStateTransition { .. })
        ));
    }

    // ========================================================================
    // 4. HAPPY-PATH LIFECYCLE SEQUENCES
    // ========================================================================

    #[test]
    fn full_lifecycle_creating_to_removed_via_ready_active_cleaning() {
        let ws = ws_creating();
        let ws = ws.mark_ready().expect("→ ready");
        assert!(ws.is_ready());
        let ws = ws.mark_active().expect("→ active");
        assert!(ws.is_active());
        let ws = ws.start_cleaning().expect("→ cleaning");
        assert!(ws.is_cleaning());
        let ws = ws.mark_removed().expect("→ removed");
        assert!(ws.is_removed());
        assert!(ws.is_terminal());
    }

    #[test]
    fn lifecycle_early_exit_creating_to_removed() {
        let ws = ws_creating().mark_removed().expect("→ removed");
        assert!(ws.is_removed());
        assert!(ws.is_terminal());
    }

    #[test]
    fn lifecycle_ready_to_removed_skipping_active() {
        let ws = ws_ready().mark_removed().expect("→ removed");
        assert!(ws.is_removed());
    }

    #[test]
    fn lifecycle_active_to_removed_skipping_cleaning() {
        let ws = ws_active().mark_removed().expect("→ removed");
        assert!(ws.is_removed());
    }

    #[test]
    fn lifecycle_ready_to_cleaning_to_removed() {
        let ws = ws_ready().start_cleaning().expect("→ cleaning");
        let ws = ws.mark_removed().expect("→ removed");
        assert!(ws.is_removed());
    }

    // ========================================================================
    // 5. QUERY METHODS — exhaustive per-state assertions
    // ========================================================================

    #[test]
    fn is_creating_only_true_for_creating() {
        assert!(ws_in_state(WorkspaceState::Creating).is_creating());
        assert!(!ws_in_state(WorkspaceState::Ready).is_creating());
        assert!(!ws_in_state(WorkspaceState::Active).is_creating());
        assert!(!ws_in_state(WorkspaceState::Cleaning).is_creating());
        assert!(!ws_in_state(WorkspaceState::Removed).is_creating());
    }

    #[test]
    fn is_ready_only_true_for_ready() {
        assert!(!ws_in_state(WorkspaceState::Creating).is_ready());
        assert!(ws_in_state(WorkspaceState::Ready).is_ready());
        assert!(!ws_in_state(WorkspaceState::Active).is_ready());
        assert!(!ws_in_state(WorkspaceState::Cleaning).is_ready());
        assert!(!ws_in_state(WorkspaceState::Removed).is_ready());
    }

    #[test]
    fn is_active_only_true_for_active() {
        assert!(!ws_in_state(WorkspaceState::Creating).is_active());
        assert!(!ws_in_state(WorkspaceState::Ready).is_active());
        assert!(ws_in_state(WorkspaceState::Active).is_active());
        assert!(!ws_in_state(WorkspaceState::Cleaning).is_active());
        assert!(!ws_in_state(WorkspaceState::Removed).is_active());
    }

    #[test]
    fn is_cleaning_only_true_for_cleaning() {
        assert!(!ws_in_state(WorkspaceState::Creating).is_cleaning());
        assert!(!ws_in_state(WorkspaceState::Ready).is_cleaning());
        assert!(!ws_in_state(WorkspaceState::Active).is_cleaning());
        assert!(ws_in_state(WorkspaceState::Cleaning).is_cleaning());
        assert!(!ws_in_state(WorkspaceState::Removed).is_cleaning());
    }

    #[test]
    fn is_removed_only_true_for_removed() {
        assert!(!ws_in_state(WorkspaceState::Creating).is_removed());
        assert!(!ws_in_state(WorkspaceState::Ready).is_removed());
        assert!(!ws_in_state(WorkspaceState::Active).is_removed());
        assert!(!ws_in_state(WorkspaceState::Cleaning).is_removed());
        assert!(ws_in_state(WorkspaceState::Removed).is_removed());
    }

    #[test]
    fn can_use_true_for_ready_and_active() {
        assert!(!ws_in_state(WorkspaceState::Creating).can_use());
        assert!(ws_in_state(WorkspaceState::Ready).can_use());
        assert!(ws_in_state(WorkspaceState::Active).can_use());
        assert!(!ws_in_state(WorkspaceState::Cleaning).can_use());
        assert!(!ws_in_state(WorkspaceState::Removed).can_use());
    }

    #[test]
    fn is_terminal_only_true_for_removed() {
        assert!(!ws_in_state(WorkspaceState::Creating).is_terminal());
        assert!(!ws_in_state(WorkspaceState::Ready).is_terminal());
        assert!(!ws_in_state(WorkspaceState::Active).is_terminal());
        assert!(!ws_in_state(WorkspaceState::Cleaning).is_terminal());
        assert!(ws_in_state(WorkspaceState::Removed).is_terminal());
    }

    #[test]
    fn query_methods_cover_all_states_exhaustively() {
        let cases: Vec<(WorkspaceState, bool, bool, bool, bool, bool, bool, bool)> = vec![
            //             state       cr  rd  ac  cl  rm  cu  tm
            (
                WorkspaceState::Creating,
                true,
                false,
                false,
                false,
                false,
                false,
                false,
            ),
            (
                WorkspaceState::Ready,
                false,
                true,
                false,
                false,
                false,
                true,
                false,
            ),
            (
                WorkspaceState::Active,
                false,
                false,
                true,
                false,
                false,
                true,
                false,
            ),
            (
                WorkspaceState::Cleaning,
                false,
                false,
                false,
                true,
                false,
                false,
                false,
            ),
            (
                WorkspaceState::Removed,
                false,
                false,
                false,
                false,
                true,
                false,
                true,
            ),
        ];
        for (state, cr, rd, ac, cl, rm, cu, tm) in cases {
            let ws = ws_in_state(state);
            assert_eq!(ws.is_creating(), cr, "is_creating for {state:?}");
            assert_eq!(ws.is_ready(), rd, "is_ready for {state:?}");
            assert_eq!(ws.is_active(), ac, "is_active for {state:?}");
            assert_eq!(ws.is_cleaning(), cl, "is_cleaning for {state:?}");
            assert_eq!(ws.is_removed(), rm, "is_removed for {state:?}");
            assert_eq!(ws.can_use(), cu, "can_use for {state:?}");
            assert_eq!(ws.is_terminal(), tm, "is_terminal for {state:?}");
        }
    }

    // ========================================================================
    // 6. VALIDATION METHODS — exhaustive per-state
    // ========================================================================

    #[test]
    fn validate_ready_ok_for_ready_and_active() {
        assert!(ws_in_state(WorkspaceState::Ready)
            .validate_ready()
            .is_ok());
        assert!(ws_in_state(WorkspaceState::Active)
            .validate_ready()
            .is_ok());
    }

    #[test]
    fn validate_ready_err_for_others() {
        for state in [WorkspaceState::Creating, WorkspaceState::Cleaning, WorkspaceState::Removed]
        {
            let ws = ws_in_state(state);
            let err = ws.validate_ready().unwrap_err();
            assert!(
                matches!(err, WorkspaceError::NotReady(s) if s == state),
                "validate_ready for {state:?}"
            );
        }
    }

    #[test]
    fn validate_active_ok_only_for_active() {
        assert!(ws_in_state(WorkspaceState::Active)
            .validate_active()
            .is_ok());
    }

    #[test]
    fn validate_active_err_for_others() {
        for state in [
            WorkspaceState::Creating,
            WorkspaceState::Ready,
            WorkspaceState::Cleaning,
            WorkspaceState::Removed,
        ] {
            let ws = ws_in_state(state);
            let err = ws.validate_active().unwrap_err();
            assert!(
                matches!(err, WorkspaceError::NotActive(s) if s == state),
                "validate_active for {state:?}"
            );
        }
    }

    #[test]
    fn validate_not_removed_ok_for_non_removed() {
        for state in [
            WorkspaceState::Creating,
            WorkspaceState::Ready,
            WorkspaceState::Active,
            WorkspaceState::Cleaning,
        ] {
            assert!(
                ws_in_state(state).validate_not_removed().is_ok(),
                "validate_not_removed for {state:?}"
            );
        }
    }

    #[test]
    fn validate_not_removed_err_for_removed() {
        let err = ws_in_state(WorkspaceState::Removed)
            .validate_not_removed()
            .unwrap_err();
        assert!(matches!(err, WorkspaceError::Removed));
    }

    #[test]
    fn validate_can_use_ok_for_ready_and_active() {
        assert!(ws_in_state(WorkspaceState::Ready)
            .validate_can_use()
            .is_ok());
        assert!(ws_in_state(WorkspaceState::Active)
            .validate_can_use()
            .is_ok());
    }

    #[test]
    fn validate_can_use_err_for_others() {
        for state in [WorkspaceState::Creating, WorkspaceState::Cleaning, WorkspaceState::Removed]
        {
            let ws = ws_in_state(state);
            let err = ws.validate_can_use().unwrap_err();
            assert!(
                matches!(err, WorkspaceError::CannotUse(s) if s == state),
                "validate_can_use for {state:?}"
            );
        }
    }

    // ========================================================================
    // 7. PATH OPERATIONS
    // ========================================================================

    #[test]
    fn change_path_to_existing_dir() {
        let ws = ws_creating();
        let new_path = PathBuf::from("/var/tmp");
        let changed = ws.change_path(new_path.clone()).expect("change");
        assert_eq!(changed.path, new_path);
        assert_eq!(changed.name, ws.name);
        assert_eq!(changed.state, ws.state);
    }

    #[test]
    fn change_path_to_nonexistent_fails() {
        let ws = ws_creating();
        let bad = PathBuf::from("/no/such/dir");
        let err = ws.change_path(bad.clone()).unwrap_err();
        match err {
            WorkspaceError::PathNotFound(p) => assert_eq!(p, bad),
            other => panic!("expected PathNotFound, got {other:?}"),
        }
    }

    #[test]
    fn change_path_preserves_state() {
        for state in WorkspaceState::all() {
            let ws = ws_in_state(state);
            let changed = ws.change_path(PathBuf::from("/tmp")).expect("change");
            assert_eq!(changed.state, state, "state preserved after change_path");
        }
    }

    #[test]
    fn change_path_preserves_name() {
        let ws = ws_creating();
        let changed = ws.change_path(PathBuf::from("/tmp")).expect("change");
        assert_eq!(changed.name, ws.name);
    }

    #[test]
    fn change_path_is_immutably_returned_original_unchanged() {
        let ws = ws_creating();
        let original_path = ws.path.clone();
        let _changed = ws.change_path(PathBuf::from("/var/tmp")).expect("change");
        // Original is unchanged (Workspace is immutable)
        assert_eq!(ws.path, original_path);
    }

    // ========================================================================
    // 8. BUILDER
    // ========================================================================

    #[test]
    fn builder_minimal_creates_in_creating_state() {
        let ws = Workspace::builder()
            .name(name("built"))
            .path(PathBuf::from("/tmp"))
            .build()
            .expect("build");
        assert_eq!(ws.name, name("built"));
        assert_eq!(ws.path, PathBuf::from("/tmp"));
        assert_eq!(ws.state, WorkspaceState::Creating);
    }

    #[test]
    fn builder_with_explicit_state() {
        for state in WorkspaceState::all() {
            let ws = Workspace::builder()
                .name(name("bs"))
                .path(PathBuf::from("/tmp"))
                .state(state)
                .build()
                .expect("build");
            assert_eq!(ws.state, state);
        }
    }

    #[test]
    fn builder_missing_name_fails() {
        let result = Workspace::builder().path(PathBuf::from("/tmp")).build();
        assert!(result.is_err());
    }

    #[test]
    fn builder_missing_path_fails() {
        let result = Workspace::builder().name(name("no-path")).build();
        assert!(result.is_err());
    }

    #[test]
    fn builder_missing_both_fails() {
        let result = Workspace::builder().build();
        assert!(result.is_err());
    }

    #[test]
    fn builder_nonexistent_path_fails() {
        let result = Workspace::builder()
            .name(name("bad"))
            .path(PathBuf::from("/nonexistent"))
            .build();
        assert!(matches!(result, Err(WorkspaceError::PathNotFound(_))));
    }

    #[test]
    fn builder_new_returns_default() {
        let b = WorkspaceBuilder::new();
        assert!(format!("{b:?}").contains("WorkspaceBuilder"));
    }

    // ========================================================================
    // 9. IMMUTABILITY & EQUALITY
    // ========================================================================

    #[test]
    fn transition_returns_new_instance_original_unchanged() {
        let ws = ws_creating();
        let original_state = ws.state;
        let _ready = ws.mark_ready().expect("→ ready");
        assert_eq!(ws.state, original_state, "original unchanged");
    }

    #[test]
    fn clone_equality() {
        let ws = ws_ready();
        let cloned = ws.clone();
        assert_eq!(ws, cloned);
    }

    #[test]
    fn different_states_not_equal() {
        assert_ne!(ws_creating(), ws_ready());
        assert_ne!(ws_ready(), ws_active());
        assert_ne!(ws_active(), ws_cleaning());
        assert_ne!(ws_cleaning(), ws_removed());
    }

    #[test]
    fn different_names_not_equal() {
        let a = Workspace::create(name("alpha"), PathBuf::from("/tmp")).expect("ok");
        let b = Workspace::create(name("beta"), PathBuf::from("/tmp")).expect("ok");
        assert_ne!(a, b);
    }

    #[test]
    fn same_name_state_path_are_equal() {
        let a = Workspace::reconstruct(name("x"), PathBuf::from("/tmp"), WorkspaceState::Ready)
            .expect("ok");
        let b = Workspace::reconstruct(name("x"), PathBuf::from("/tmp"), WorkspaceState::Ready)
            .expect("ok");
        assert_eq!(a, b);
    }

    // ========================================================================
    // 10. ERROR DISPLAY
    // ========================================================================

    #[test]
    fn error_displays() {
        let err = WorkspaceError::InvalidStateTransition {
            from: WorkspaceState::Creating,
            to: WorkspaceState::Active,
        };
        let msg = format!("{err}");
        assert!(msg.contains("Creating"));
        assert!(msg.contains("Active"));

        let err = WorkspaceError::PathNotFound(PathBuf::from("/bad"));
        assert!(format!("{err}").contains("/bad"));

        let err = WorkspaceError::NotReady(WorkspaceState::Creating);
        assert!(format!("{err}").contains("not ready"));

        let err = WorkspaceError::NotActive(WorkspaceState::Ready);
        assert!(format!("{err}").contains("not active"));

        let err = WorkspaceError::Removed;
        assert!(format!("{err}").contains("removed"));

        let err = WorkspaceError::CannotUse(WorkspaceState::Cleaning);
        assert!(format!("{err}").contains("cannot use"));

        let err = WorkspaceError::NameAlreadyExists(name("dup"));
        assert!(format!("{err}").contains("dup"));
    }

    #[test]
    fn error_equality() {
        // WorkspaceError derives PartialEq
        assert_eq!(
            WorkspaceError::Removed,
            WorkspaceError::Removed
        );
        assert_eq!(
            WorkspaceError::PathNotFound(PathBuf::from("/a")),
            WorkspaceError::PathNotFound(PathBuf::from("/a"))
        );
        assert_ne!(
            WorkspaceError::PathNotFound(PathBuf::from("/a")),
            WorkspaceError::PathNotFound(PathBuf::from("/b"))
        );
    }

    // ========================================================================
    // 11. WORKSPACE METADATA — name preserved through all operations
    // ========================================================================

    #[test]
    fn name_preserved_through_transitions() {
        let ws = ws_creating();
        let ws = ws.mark_ready().expect("ok");
        let ws = ws.mark_active().expect("ok");
        let ws = ws.start_cleaning().expect("ok");
        let ws = ws.mark_removed().expect("ok");
        assert_eq!(ws.name, name("test-ws"));
    }

    #[test]
    fn path_preserved_through_transitions() {
        let ws = ws_creating();
        let ws = ws.mark_ready().expect("ok");
        let ws = ws.mark_active().expect("ok");
        assert_eq!(ws.path, PathBuf::from("/tmp"));
    }

    // ========================================================================
    // 12. WORKSPACE STATE ENUM HELPERS (domain::workspace::WorkspaceState)
    // ========================================================================

    #[test]
    fn state_all_has_five_variants() {
        assert_eq!(WorkspaceState::all().len(), 5);
    }

    #[test]
    fn state_all_variants_distinct() {
        let mut seen = std::collections::HashSet::new();
        for s in WorkspaceState::all() {
            assert!(seen.insert(s), "duplicate state: {s:?}");
        }
    }

    #[test]
    fn state_display_lowercase() {
        assert_eq!(WorkspaceState::Creating.to_string(), "creating");
        assert_eq!(WorkspaceState::Ready.to_string(), "ready");
        assert_eq!(WorkspaceState::Active.to_string(), "active");
        assert_eq!(WorkspaceState::Cleaning.to_string(), "cleaning");
        assert_eq!(WorkspaceState::Removed.to_string(), "removed");
    }

    #[test]
    fn state_is_active_helper() {
        assert!(WorkspaceState::Active.is_active());
        assert!(!WorkspaceState::Ready.is_active());
        assert!(!WorkspaceState::Creating.is_active());
    }

    #[test]
    fn state_is_ready_helper() {
        assert!(WorkspaceState::Ready.is_ready());
        assert!(WorkspaceState::Active.is_ready());
        assert!(!WorkspaceState::Creating.is_ready());
    }

    #[test]
    fn state_valid_transitions_completeness() {
        // Creating → [Ready, Removed]
        let t = WorkspaceState::Creating.valid_transitions();
        assert_eq!(t.len(), 2);
        assert!(t.contains(&WorkspaceState::Ready));
        assert!(t.contains(&WorkspaceState::Removed));

        // Ready → [Active, Cleaning, Removed]
        let t = WorkspaceState::Ready.valid_transitions();
        assert_eq!(t.len(), 3);

        // Active → [Cleaning, Removed]
        let t = WorkspaceState::Active.valid_transitions();
        assert_eq!(t.len(), 2);

        // Cleaning → [Removed]
        let t = WorkspaceState::Cleaning.valid_transitions();
        assert_eq!(t.len(), 1);

        // Removed → []
        assert!(WorkspaceState::Removed.valid_transitions().is_empty());
    }

    #[test]
    fn state_is_terminal_only_removed() {
        assert!(WorkspaceState::Removed.is_terminal());
        assert!(!WorkspaceState::Creating.is_terminal());
        assert!(!WorkspaceState::Ready.is_terminal());
        assert!(!WorkspaceState::Active.is_terminal());
        assert!(!WorkspaceState::Cleaning.is_terminal());
    }

    #[test]
    fn state_copy_works() {
        let a = WorkspaceState::Active;
        let b = a;
        assert_eq!(a, b);
    }

    // ========================================================================
    // 13. PROPTESTS — property-based invariants
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 256,
            .. ProptestConfig::default()
        })]

        /// Creating always produces Creating state
        #[test]
        fn proptest_create_always_creating(name_str in "[a-zA-Z0-9_-]{1,20}") {
            let ws = Workspace::create(
                WorkspaceName::parse(&name_str).unwrap_or_else(|_| WorkspaceName::parse("fallback").unwrap()),
                PathBuf::from("/tmp"),
            );
            prop_assert!(ws.is_ok());
            prop_assert!(ws.unwrap().is_creating());
        }

        /// Transition always returns a new workspace with target state (for valid pairs)
        #[test]
        fn proptest_valid_transition_changes_state(
            from_idx in 0usize..5,
            to_idx in 0usize..5,
        ) {
            let states = WorkspaceState::all();
            let from = states[from_idx];
            let to = states[to_idx];

            if !from.can_transition_to(&to) {
                // Invalid pair — skip (tested separately)
                return Ok(());
            }

            let ws = ws_in_state(from);
            let result = match to {
                WorkspaceState::Ready => ws.mark_ready(),
                WorkspaceState::Active => ws.mark_active(),
                WorkspaceState::Cleaning => ws.start_cleaning(),
                WorkspaceState::Removed => ws.mark_removed(),
                WorkspaceState::Creating => ws.mark_ready(), // unreachable
            };

            prop_assert!(result.is_ok());
            prop_assert_eq!(result.unwrap().state, to);
        }

        /// Invalid transitions always return InvalidStateTransition
        /// (skips Creating target since no public method targets it)
        #[test]
        fn proptest_invalid_transition_returns_error(
            from_idx in 0usize..5,
            to_idx in 1usize..5, // skip Creating (index 0) — no method targets it
        ) {
            let states = WorkspaceState::all();
            let from = states[from_idx];
            let to = states[to_idx];

            if from.can_transition_to(&to) {
                return Ok(());
            }

            let ws = ws_in_state(from);
            let result = match to {
                WorkspaceState::Ready => ws.mark_ready(),
                WorkspaceState::Active => ws.mark_active(),
                WorkspaceState::Cleaning => ws.start_cleaning(),
                WorkspaceState::Removed => ws.mark_removed(),
                WorkspaceState::Creating => unreachable!(),
            };

            let is_invalid = match result {
                Err(WorkspaceError::InvalidStateTransition { .. }) => true,
                _ => false,
            };
            prop_assert!(is_invalid);
        }

        /// Name is always preserved through any transition
        #[test]
        fn proptest_name_preserved_through_transition(from_idx in 0usize..4) {
            let states = WorkspaceState::all();
            let from = states[from_idx];
            let ws = ws_in_state(from);
            let valid_targets = from.valid_transitions();

            if valid_targets.is_empty() {
                return Ok(());
            }

            let target = valid_targets[0];
            let result = match target {
                WorkspaceState::Ready => ws.mark_ready(),
                WorkspaceState::Active => ws.mark_active(),
                WorkspaceState::Cleaning => ws.start_cleaning(),
                WorkspaceState::Removed => ws.mark_removed(),
                WorkspaceState::Creating => ws.mark_ready(),
            };

            prop_assert!(result.is_ok());
            prop_assert_eq!(result.unwrap().name, ws.name);
        }

        /// change_path to /tmp always succeeds and preserves name+state
        #[test]
        fn proptest_change_path_preserves_identity(state_idx in 0usize..5) {
            let states = WorkspaceState::all();
            let ws = ws_in_state(states[state_idx]);
            let changed = ws.change_path(PathBuf::from("/tmp"));
            prop_assert!(changed.is_ok());
            let changed = changed.unwrap();
            prop_assert_eq!(changed.name, ws.name);
            prop_assert_eq!(changed.state, ws.state);
        }

        /// Reconstruct round-trip: state in → reconstruct → check state
        #[test]
        fn proptest_reconstruct_preserves_state(state_idx in 0usize..5) {
            let states = WorkspaceState::all();
            let state = states[state_idx];
            let ws = Workspace::reconstruct(name("rt"), PathBuf::from("/tmp"), state);
            prop_assert!(ws.is_ok());
            prop_assert_eq!(ws.unwrap().state, state);
        }
    }
}
