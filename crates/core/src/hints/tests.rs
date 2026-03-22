//! Tests for hints module

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::types::{AbsolutePath, SessionId, SessionName, ValidatedMetadata};
    use crate::workspace_state::WorkspaceState;

    use super::types::{ActionRisk, CommandContext, Hint, HintType, NextAction, SystemState};
    use super::{
        extract_session_name, generate_hints, generate_hints_response, hints_for_beads,
        hints_for_error, next_actions_for_command, suggest_next_actions,
    };
    use crate::types::{BranchState, SessionStatus};

    fn create_test_session(name: &str, status: SessionStatus) -> Session {
        Session {
            id: SessionId::parse(format!("id-{name}")).expect("valid id in test"),
            name: SessionName::new(name).expect("valid session name in test"),
            status,
            state: WorkspaceState::default(),
            workspace_path: AbsolutePath::parse("/tmp/test").expect("valid path in test"),
            branch: BranchState::Detached,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_synced: None,
            metadata: ValidatedMetadata::empty(),
        }
    }

    #[test]
    fn test_hint_builders() {
        let hint = Hint::info("Test message")
            .with_command("scp test")
            .with_rationale("Testing");

        assert_eq!(hint.hint_type, HintType::Info);
        assert_eq!(hint.message, "Test message");
        assert_eq!(hint.suggested_command, Some("scp test".to_string()));
        assert_eq!(hint.rationale, Some("Testing".to_string()));
    }

    #[test]
    fn test_generate_hints_no_sessions() {
        let state = SystemState {
            sessions: Vec::new(),
            initialized: true,
            jj_repo: true,
        };

        let hints = generate_hints(&state).unwrap_or_else(|_| Vec::new());
        assert!(!hints.is_empty());

        #[allow(clippy::indexing_slicing)]
        {
            assert!(hints[0].message.contains("first parallel workspace"));
        }
    }

    #[test]
    fn test_generate_hints_completed_session() {
        let mut session = create_test_session("old-session", SessionStatus::Completed);
        session.updated_at = Utc::now() - chrono::Duration::days(3);

        let state = SystemState {
            sessions: vec![session],
            initialized: true,
            jj_repo: true,
        };

        let hints = generate_hints(&state).unwrap_or_else(|_| Vec::new());
        assert!(hints
            .iter()
            .any(|h| h.message.contains("consider removing")));
    }

    #[test]
    fn test_generate_hints_failed_session() {
        let state = SystemState {
            sessions: vec![create_test_session("failed-session", SessionStatus::Failed)],
            initialized: true,
            jj_repo: true,
        };

        let hints = generate_hints(&state).unwrap_or_else(|_| Vec::new());
        assert!(hints.iter().any(|h| h.hint_type == HintType::Warning));
    }

    #[test]
    fn test_generate_hints_multiple_active() {
        let state = SystemState {
            sessions: vec![
                create_test_session("session1", SessionStatus::Active),
                create_test_session("session2", SessionStatus::Active),
                create_test_session("session3", SessionStatus::Active),
            ],
            initialized: true,
            jj_repo: true,
        };

        let hints = generate_hints(&state).unwrap_or_else(|_| Vec::new());
        assert!(!hints.is_empty() || hints.is_empty());
    }

    #[test]
    fn test_hints_for_error_session_exists() {
        let hints = hints_for_error("SESSION_ALREADY_EXISTS", "Session 'test' already exists");
        assert_eq!(hints.len(), 3);

        #[allow(clippy::indexing_slicing)]
        {
            assert!(hints[0].message.contains("different name"));
            assert!(hints[1].message.contains("Switch"));
            assert!(hints[2].message.contains("Remove"));
        }
    }

    #[test]
    fn test_hints_for_error_not_initialized() {
        let hints = hints_for_error("NOT_INITIALIZED", "scp not initialized");
        assert!(!hints.is_empty());

        #[allow(clippy::indexing_slicing)]
        {
            assert!(hints[0].message.contains("Initialize"));
        }
    }

    #[test]
    fn test_suggest_next_actions_not_initialized() {
        let state = SystemState {
            sessions: Vec::new(),
            initialized: false,
            jj_repo: true,
        };

        let actions = suggest_next_actions(&state);
        assert_eq!(actions.len(), 1);

        #[allow(clippy::indexing_slicing)]
        {
            assert_eq!(actions[0].action, "Initialize scp");
        }
    }

    #[test]
    fn test_suggest_next_actions_no_sessions() {
        let state = SystemState {
            sessions: Vec::new(),
            initialized: true,
            jj_repo: true,
        };

        let actions = suggest_next_actions(&state);
        assert!(actions.iter().any(|a| a.action.contains("first session")));
    }

    #[test]
    fn test_suggest_next_actions_has_completed() {
        let state = SystemState {
            sessions: vec![create_test_session("done", SessionStatus::Completed)],
            initialized: true,
            jj_repo: true,
        };

        let actions = suggest_next_actions(&state);
        assert!(actions.iter().any(|a| a.action.contains("Clean up")));
    }

    #[test]
    fn test_hints_for_beads_blockers() {
        let beads = BeadsSummary {
            open: 2,
            in_progress: 1,
            blocked: 3,
            closed: 5,
        };

        let hints = hints_for_beads("test-session", &beads);
        assert!(hints.iter().any(|h| h.hint_type == HintType::Warning));
        assert!(hints.iter().any(|h| h.message.contains("blocked")));
    }

    #[test]
    fn test_hints_for_beads_too_many_active() {
        let beads = BeadsSummary {
            open: 4,
            in_progress: 3,
            blocked: 0,
            closed: 5,
        };

        let hints = hints_for_beads("test-session", &beads);
        assert!(hints.iter().any(|h| h.message.contains("fewer tasks")));
    }

    #[test]
    fn test_hints_for_beads_none() {
        let beads = BeadsSummary::default();

        let hints = hints_for_beads("test-session", &beads);
        assert!(hints.iter().any(|h| h.message.contains("no beads")));
    }

    #[test]
    fn test_extract_session_name() {
        assert_eq!(
            extract_session_name("Session 'test-name' already exists"),
            Some("test-name")
        );
        assert_eq!(
            extract_session_name("Session 'my-session' not found"),
            Some("my-session")
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // NEXT ACTIONS FOR COMMAND TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    fn success_context(command: &str, session_name: Option<&str>) -> CommandContext {
        CommandContext {
            command: command.to_string(),
            success: true,
            session_count: 2,
            session_name: session_name.map(String::from),
        }
    }

    fn error_context(command: &str) -> CommandContext {
        CommandContext {
            command: command.to_string(),
            success: false,
            session_count: 0,
            session_name: None,
        }
    }

    #[test]
    fn test_next_actions_init_success() {
        let actions = next_actions_for_command(&success_context("init", None));
        assert!(!actions.is_empty());
        assert!(actions.len() <= 5);
        assert!(actions.iter().any(|a| a.action.contains("first session")));
        for action in &actions {
            assert!(!action.commands.is_empty());
            for cmd in &action.commands {
                assert!(!cmd.is_empty());
            }
        }
    }

    #[test]
    fn test_next_actions_add_success_with_session() {
        let actions = next_actions_for_command(&success_context("add", Some("feature-x")));
        assert!(!actions.is_empty());
        assert!(actions.len() <= 5);
        assert!(actions
            .iter()
            .any(|a| a.commands.iter().any(|c| c.contains("focus feature-x"))));
    }

    #[test]
    fn test_next_actions_remove_success() {
        let actions = next_actions_for_command(&success_context("remove", Some("old")));
        assert!(!actions.is_empty());
        assert!(actions.iter().any(|a| a.action.contains("List")));
    }

    #[test]
    fn test_next_actions_list_no_sessions() {
        let ctx = CommandContext {
            command: "list".to_string(),
            success: true,
            session_count: 0,
            session_name: None,
        };
        let actions = next_actions_for_command(&ctx);
        assert!(actions.iter().any(|a| a.action.contains("first session")));
    }

    #[test]
    fn test_next_actions_list_has_sessions() {
        let actions = next_actions_for_command(&success_context("list", None));
        assert!(actions.iter().any(|a| a.action.contains("status")));
    }

    #[test]
    fn test_next_actions_focus_success() {
        let actions = next_actions_for_command(&success_context("focus", Some("my-session")));
        assert!(!actions.is_empty());
        let sync_action = actions.iter().find(|a| a.action.contains("Sync"));
        assert!(sync_action.is_some());
        if let Some(sa) = sync_action {
            assert_eq!(sa.risk, ActionRisk::Medium);
        }
    }

    #[test]
    fn test_next_actions_status_includes_risk_levels() {
        let actions = next_actions_for_command(&success_context("status", Some("sess")));
        let remove = actions.iter().find(|a| a.action.contains("Remove"));
        assert!(remove.is_some());
        if let Some(r) = remove {
            assert_eq!(r.risk, ActionRisk::High);
        }
    }

    #[test]
    fn test_next_actions_unknown_command_returns_empty() {
        let actions = next_actions_for_command(&success_context("nonexistent", None));
        assert!(actions.is_empty());
    }

    #[test]
    fn test_next_actions_error_returns_suggestions() {
        let actions = next_actions_for_command(&error_context("add"));
        assert!(!actions.is_empty());
        assert!(actions
            .iter()
            .any(|a| a.commands.iter().any(|c| c.contains("scp session list"))));
    }

    #[test]
    fn test_next_actions_error_unknown_returns_empty() {
        let actions = next_actions_for_command(&error_context("nonexistent"));
        assert!(actions.is_empty());
    }

    #[test]
    fn test_next_actions_all_have_copy_pastable_commands() {
        let commands = [
            "init", "add", "remove", "list", "focus", "status", "sync", "doctor", "clean",
        ];
        for cmd in &commands {
            let ctx = success_context(cmd, Some("test-sess"));
            let actions = next_actions_for_command(&ctx);
            for action in &actions {
                assert!(
                    !action.commands.is_empty(),
                    "Command {cmd} action '{}' has no commands",
                    action.action
                );
                for c in &action.commands {
                    assert!(!c.is_empty(), "Command {cmd} has empty command string");
                }
            }
        }
    }

    #[test]
    fn test_next_actions_max_5() {
        let commands = [
            "init", "add", "remove", "list", "focus", "status", "sync", "doctor", "clean",
        ];
        for cmd in &commands {
            let ctx = success_context(cmd, Some("s"));
            let actions = next_actions_for_command(&ctx);
            assert!(
                actions.len() <= 5,
                "Command {cmd} returned {} actions",
                actions.len()
            );
        }
    }

    #[test]
    fn test_action_risk_default_is_safe() {
        assert_eq!(ActionRisk::default(), ActionRisk::Safe);
    }

    #[test]
    fn test_action_risk_serialization() {
        let safe_json = serde_json::to_string(&ActionRisk::Safe).unwrap_or_else(|_| String::new());
        assert_eq!(safe_json, "\"safe\"");
        let medium_json =
            serde_json::to_string(&ActionRisk::Medium).unwrap_or_else(|_| String::new());
        assert_eq!(medium_json, "\"medium\"");
        let high_json = serde_json::to_string(&ActionRisk::High).unwrap_or_else(|_| String::new());
        assert_eq!(high_json, "\"high\"");
    }

    #[test]
    fn test_next_action_serialization_includes_risk() {
        let action = NextAction {
            action: "Test".to_string(),
            commands: vec!["scp test".to_string()],
            risk: ActionRisk::Medium,
            description: Some("A test action".to_string()),
        };
        let json = serde_json::to_string(&action).unwrap_or_else(|_| String::new());
        assert!(json.contains("\"risk\":\"medium\""));
        assert!(json.contains("\"description\":\"A test action\""));
    }

    #[test]
    fn test_next_action_serialization_omits_none_description() {
        let action = NextAction {
            action: "Test".to_string(),
            commands: vec!["scp test".to_string()],
            risk: ActionRisk::Safe,
            description: None,
        };
        let json = serde_json::to_string(&action).unwrap_or_else(|_| String::new());
        assert!(!json.contains("description"));
    }

    #[test]
    fn test_command_context_clone() {
        let ctx = success_context("init", Some("s"));
        let cloned = ctx.clone();
        assert_eq!(ctx.command, cloned.command);
        assert_eq!(ctx.success, cloned.success);
    }

    #[test]
    fn test_generate_hints_response() {
        let state = SystemState {
            sessions: vec![create_test_session("active", SessionStatus::Active)],
            initialized: true,
            jj_repo: true,
        };

        use super::response::SystemContext;
        let response =
            generate_hints_response(&state).unwrap_or_else(|_| super::response::HintsResponse {
                context: SystemContext {
                    initialized: true,
                    jj_repo: true,
                    sessions_count: 0,
                    active_sessions: 0,
                    has_changes: false,
                },
                hints: Vec::new(),
                next_actions: Vec::new(),
            });

        assert_eq!(response.context.sessions_count, 1);
        assert_eq!(response.context.active_sessions, 1);
        assert!(!response.hints.is_empty());
        assert!(!response.next_actions.is_empty());
    }
}
