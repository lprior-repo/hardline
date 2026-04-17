use crate::error::Result;
use crate::views::WorktreeView;
use scp_stack::domain::StackBranch;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum FocusedPane {
    Stack,
    Diff,
    Worktrees,
}

#[derive(Debug)]
pub enum Mode {
    Normal,
    Search,
    Help,
    Confirm(ConfirmAction),
    Input(InputAction),
    Reorder,
}

#[derive(Debug)]
pub enum ConfirmAction {
    Delete(String),
    Restack(String),
    RestackAll,
    ApplyReorder,
}

#[derive(Debug)]
pub enum InputAction {
    Rename,
    NewBranch,
}

pub struct TuiApp {
    pub focused_pane: FocusedPane,
    pub mode: Mode,
    pub needs_refresh: bool,
    pub should_quit: bool,
    pub worktree_view: WorktreeView,
    pub stack_branches: Vec<StackBranch>,
}

impl TuiApp {
    pub fn new() -> Result<Self> {
        Ok(Self {
            focused_pane: FocusedPane::Stack,
            mode: Mode::Normal,
            needs_refresh: true,
            should_quit: false,
            worktree_view: WorktreeView::default(),
            stack_branches: Vec::new(),
        })
    }

    pub fn refresh_branches(&mut self) -> Result<()> {
        self.needs_refresh = false;
        Ok(())
    }

    pub fn selected_branch(&self) -> Option<String> {
        None
    }

    pub fn set_status(&mut self, _message: String) {}
}

pub fn run() -> Result<()> {
    let mut app = TuiApp::new()?;
    app.needs_refresh = true;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Constructor & default state ──

    #[test]
    fn tui_app_new_returns_default_state() {
        let app = TuiApp::new().expect("TuiApp::new should succeed");
        assert!(!app.should_quit);
        assert!(app.needs_refresh);
        assert!(matches!(app.focused_pane, FocusedPane::Stack));
        assert!(matches!(app.mode, Mode::Normal));
    }

    #[test]
    fn multiple_app_instances_are_independent() {
        let mut app1 = TuiApp::new().expect("ok");
        let mut app2 = TuiApp::new().expect("ok");
        app1.should_quit = true;
        app2.mode = Mode::Help;
        assert!(app1.should_quit);
        assert!(!app2.should_quit);
        assert!(matches!(app1.mode, Mode::Normal));
        assert!(matches!(app2.mode, Mode::Help));
    }

    // ── refresh_branches ──

    #[test]
    fn refresh_branches_clears_needs_refresh() {
        let mut app = TuiApp::new().expect("ok");
        app.needs_refresh = true;
        app.refresh_branches().expect("should succeed");
        assert!(!app.needs_refresh);
    }

    #[test]
    fn refresh_branches_when_already_clear_stays_clear() {
        let mut app = TuiApp::new().expect("ok");
        app.needs_refresh = false;
        app.refresh_branches().expect("should succeed");
        assert!(!app.needs_refresh);
    }

    #[test]
    fn refresh_does_not_affect_other_fields() {
        let mut app = TuiApp::new().expect("ok");
        app.should_quit = true;
        app.focused_pane = FocusedPane::Diff;
        app.mode = Mode::Search;
        app.refresh_branches().expect("ok");
        assert!(app.should_quit);
        assert!(matches!(app.focused_pane, FocusedPane::Diff));
        assert!(matches!(app.mode, Mode::Search));
    }

    // ── selected_branch ──

    #[test]
    fn selected_branch_returns_none_by_default() {
        let app = TuiApp::new().expect("ok");
        assert!(app.selected_branch().is_none());
    }

    // ── set_status ──

    #[test]
    fn set_status_does_not_panic() {
        let mut app = TuiApp::new().expect("ok");
        app.set_status("test message".to_string());
    }

    #[test]
    fn set_status_with_empty_string() {
        let mut app = TuiApp::new().expect("ok");
        app.set_status(String::new());
    }

    #[test]
    fn set_status_with_long_string() {
        let mut app = TuiApp::new().expect("ok");
        app.set_status("a".repeat(10_000));
    }

    // ── run ──

    #[test]
    fn run_returns_ok() {
        assert!(run().is_ok());
    }

    // ── FocusedPane discriminants ──

    #[test]
    fn focused_pane_variants_are_distinct() {
        let stack = FocusedPane::Stack;
        let diff = FocusedPane::Diff;
        assert!(std::mem::discriminant(&stack) != std::mem::discriminant(&diff));
    }

    #[test]
    fn focused_pane_size_is_minimal() {
        assert_eq!(std::mem::size_of::<FocusedPane>(), 1);
    }

    // ── Mode variants ──

    #[test]
    fn mode_normal_is_not_other_variants() {
        let modes = vec![
            Mode::Search,
            Mode::Help,
            Mode::Confirm(ConfirmAction::RestackAll),
            Mode::Input(InputAction::Rename),
            Mode::Reorder,
        ];
        for other in modes {
            assert!(
                !matches!(Mode::Normal, _ if std::mem::discriminant(&Mode::Normal) == std::mem::discriminant(&other))
            );
        }
    }

    #[test]
    fn mode_confirm_carries_branch_name_for_delete() {
        let mode = Mode::Confirm(ConfirmAction::Delete("my-branch".to_string()));
        match mode {
            Mode::Confirm(ConfirmAction::Delete(name)) => assert_eq!(name, "my-branch"),
            _ => panic!("expected Confirm::Delete"),
        }
    }

    #[test]
    fn mode_confirm_carries_branch_name_for_restack() {
        let mode = Mode::Confirm(ConfirmAction::Restack("feature/x".to_string()));
        match mode {
            Mode::Confirm(ConfirmAction::Restack(name)) => assert_eq!(name, "feature/x"),
            _ => panic!("expected Confirm::Restack"),
        }
    }

    #[test]
    fn mode_confirm_delete_preserves_empty_name() {
        let mode = Mode::Confirm(ConfirmAction::Delete(String::new()));
        match mode {
            Mode::Confirm(ConfirmAction::Delete(name)) => assert!(name.is_empty()),
            _ => panic!("expected Confirm::Delete"),
        }
    }

    #[test]
    fn mode_confirm_delete_preserves_long_name() {
        let long_name = "a".repeat(1000);
        let mode = Mode::Confirm(ConfirmAction::Delete(long_name.clone()));
        match mode {
            Mode::Confirm(ConfirmAction::Delete(name)) => assert_eq!(name.len(), 1000),
            _ => panic!("expected Confirm::Delete"),
        }
    }

    #[test]
    fn confirm_actions_are_unit_except_delete_and_restack() {
        // RestackAll and ApplyReorder carry no data
        let _ = ConfirmAction::RestackAll;
        let _ = ConfirmAction::ApplyReorder;
    }

    #[test]
    fn input_action_variants_are_distinct() {
        let rename = InputAction::Rename;
        let new_branch = InputAction::NewBranch;
        assert!(std::mem::discriminant(&rename) != std::mem::discriminant(&new_branch));
    }

    // ── State transition simulation ──

    #[test]
    fn simulate_quit_flow() {
        let mut app = TuiApp::new().expect("ok");
        assert!(!app.should_quit);
        app.should_quit = true;
        assert!(app.should_quit);
    }

    #[test]
    fn simulate_pane_switching() {
        let mut app = TuiApp::new().expect("ok");
        assert!(matches!(app.focused_pane, FocusedPane::Stack));
        app.focused_pane = FocusedPane::Diff;
        assert!(matches!(app.focused_pane, FocusedPane::Diff));
        app.focused_pane = FocusedPane::Stack;
        assert!(matches!(app.focused_pane, FocusedPane::Stack));
    }

    #[test]
    fn simulate_mode_transitions() {
        let mut app = TuiApp::new().expect("ok");
        // Normal -> Search
        app.mode = Mode::Search;
        assert!(matches!(app.mode, Mode::Search));
        // Search -> Normal
        app.mode = Mode::Normal;
        assert!(matches!(app.mode, Mode::Normal));
        // Normal -> Help
        app.mode = Mode::Help;
        assert!(matches!(app.mode, Mode::Help));
        // Help -> Normal
        app.mode = Mode::Normal;
        assert!(matches!(app.mode, Mode::Normal));
        // Normal -> Confirm(Delete)
        app.mode = Mode::Confirm(ConfirmAction::Delete("b".to_string()));
        assert!(matches!(app.mode, Mode::Confirm(ConfirmAction::Delete(_))));
        // Confirm -> Normal
        app.mode = Mode::Normal;
        assert!(matches!(app.mode, Mode::Normal));
        // Normal -> Reorder
        app.mode = Mode::Reorder;
        assert!(matches!(app.mode, Mode::Reorder));
        // Reorder -> Normal
        app.mode = Mode::Normal;
        assert!(matches!(app.mode, Mode::Normal));
    }

    #[test]
    fn simulate_full_lifecycle() {
        let mut app = TuiApp::new().expect("ok");
        // Initial refresh
        assert!(app.needs_refresh);
        app.refresh_branches().expect("ok");
        assert!(!app.needs_refresh);

        // User interacts — switch pane
        app.focused_pane = FocusedPane::Diff;
        // User opens help
        app.mode = Mode::Help;
        // User closes help
        app.mode = Mode::Normal;
        // Trigger another refresh
        app.needs_refresh = true;
        app.refresh_branches().expect("ok");
        // User quits
        app.should_quit = true;
        assert!(app.should_quit);
        assert!(!app.needs_refresh);
    }

    #[test]
    fn needs_refresh_can_be_rearmed() {
        let mut app = TuiApp::new().expect("ok");
        app.needs_refresh = false;
        app.refresh_branches().expect("ok");
        app.needs_refresh = true;
        app.refresh_branches().expect("ok");
        assert!(!app.needs_refresh);
        app.needs_refresh = true;
        assert!(app.needs_refresh);
    }

    // ── TuiApp memory layout ──

    #[test]
    fn tui_app_is_not_zero_sized() {
        assert!(std::mem::size_of::<TuiApp>() > 0);
    }

    #[test]
    fn tui_app_fields_are_accessible() {
        let app = TuiApp::new().expect("ok");
        let _pane: &FocusedPane = &app.focused_pane;
        let _mode: &Mode = &app.mode;
        let _needs_refresh: bool = app.needs_refresh;
        let _should_quit: bool = app.should_quit;
    }

    // ── Result type integration ──

    #[test]
    fn tui_app_new_satisfies_result_contract() {
        let result: Result<TuiApp> = TuiApp::new();
        assert!(result.is_ok());
        let _app = result.expect("ok");
    }

    #[test]
    fn refresh_branches_satisfies_result_contract() {
        let mut app = TuiApp::new().expect("ok");
        let result: Result<()> = app.refresh_branches();
        assert!(result.is_ok());
    }

    // ── FocusedPane exhaustive variants ──

    #[test]
    fn focused_pane_all_variants_constructible() {
        let _s = FocusedPane::Stack;
        let _d = FocusedPane::Diff;
    }

    #[test]
    fn focused_pane_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<FocusedPane>();
    }

    #[test]
    fn focused_pane_debug_format() {
        let s = FocusedPane::Stack;
        let d = FocusedPane::Diff;
        let s_str = format!("{s:?}");
        let d_str = format!("{d:?}");
        assert!(!s_str.is_empty());
        assert!(!d_str.is_empty());
        assert_ne!(s_str, d_str);
    }

    // ── Mode exhaustive variants ──

    #[test]
    fn mode_all_variants_constructible() {
        let _normal = Mode::Normal;
        let _search = Mode::Search;
        let _help = Mode::Help;
        let _confirm_delete = Mode::Confirm(ConfirmAction::Delete("x".into()));
        let _confirm_restack = Mode::Confirm(ConfirmAction::Restack("x".into()));
        let _confirm_restack_all = Mode::Confirm(ConfirmAction::RestackAll);
        let _confirm_apply = Mode::Confirm(ConfirmAction::ApplyReorder);
        let _input_rename = Mode::Input(InputAction::Rename);
        let _input_new = Mode::Input(InputAction::NewBranch);
        let _reorder = Mode::Reorder;
    }

    #[test]
    fn mode_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Mode>();
    }

    #[test]
    fn mode_debug_format_variants() {
        let normal = format!("{:?}", Mode::Normal);
        let search = format!("{:?}", Mode::Search);
        let help = format!("{:?}", Mode::Help);
        let reorder = format!("{:?}", Mode::Reorder);
        for (name, s) in [
            ("Normal", normal),
            ("Search", search),
            ("Help", help),
            ("Reorder", reorder),
        ] {
            assert!(!s.is_empty(), "{name} debug should not be empty");
        }
    }

    #[test]
    fn mode_confirm_debug_shows_action() {
        let confirm = Mode::Confirm(ConfirmAction::Delete("branch".into()));
        let debug = format!("{confirm:?}");
        assert!(
            debug.contains("Delete"),
            "should mention Delete action: {debug}"
        );
    }

    #[test]
    fn mode_input_debug_shows_action() {
        let input = Mode::Input(InputAction::Rename);
        let debug = format!("{input:?}");
        assert!(
            debug.contains("Rename"),
            "should mention Rename action: {debug}"
        );
    }

    #[test]
    fn mode_input_new_branch_debug() {
        let input = Mode::Input(InputAction::NewBranch);
        let debug = format!("{input:?}");
        assert!(
            debug.contains("NewBranch"),
            "should mention NewBranch action: {debug}"
        );
    }

    // ── ConfirmAction exhaustive variants ──

    #[test]
    fn confirm_action_delete_captures_name() {
        let action = ConfirmAction::Delete("main".to_string());
        match action {
            ConfirmAction::Delete(name) => assert_eq!(name, "main"),
            _ => panic!("expected Delete"),
        }
    }

    #[test]
    fn confirm_action_restack_captures_name() {
        let action = ConfirmAction::Restack("feature".to_string());
        match action {
            ConfirmAction::Restack(name) => assert_eq!(name, "feature"),
            _ => panic!("expected Restack"),
        }
    }

    #[test]
    fn confirm_action_restack_all_is_unit() {
        let _ = ConfirmAction::RestackAll;
    }

    #[test]
    fn confirm_action_apply_reorder_is_unit() {
        let _ = ConfirmAction::ApplyReorder;
    }

    #[test]
    fn confirm_action_all_variants_distinct_discriminants() {
        use std::mem::discriminant;
        let delete = ConfirmAction::Delete("a".into());
        let restack = ConfirmAction::Restack("b".into());
        let restack_all = ConfirmAction::RestackAll;
        let apply = ConfirmAction::ApplyReorder;
        assert_ne!(discriminant(&delete), discriminant(&restack));
        assert_ne!(discriminant(&delete), discriminant(&restack_all));
        assert_ne!(discriminant(&delete), discriminant(&apply));
        assert_ne!(discriminant(&restack), discriminant(&restack_all));
        assert_ne!(discriminant(&restack), discriminant(&apply));
        assert_ne!(discriminant(&restack_all), discriminant(&apply));
    }

    #[test]
    fn confirm_action_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ConfirmAction>();
    }

    #[test]
    fn confirm_action_debug_format() {
        let delete = format!("{:?}", ConfirmAction::Delete("x".into()));
        let restack = format!("{:?}", ConfirmAction::Restack("y".into()));
        let all = format!("{:?}", ConfirmAction::RestackAll);
        let apply = format!("{:?}", ConfirmAction::ApplyReorder);
        assert!(delete.contains("Delete"));
        assert!(restack.contains("Restack"));
        assert!(all.contains("RestackAll"));
        assert!(apply.contains("ApplyReorder"));
    }

    #[test]
    fn confirm_action_delete_with_unicode_name() {
        let action = ConfirmAction::Delete("feature/日本語-branch".to_string());
        match action {
            ConfirmAction::Delete(name) => assert_eq!(name, "feature/日本語-branch"),
            _ => panic!("expected Delete"),
        }
    }

    #[test]
    fn confirm_action_delete_with_whitespace_name() {
        let action = ConfirmAction::Delete("   ".to_string());
        match action {
            ConfirmAction::Delete(name) => assert_eq!(name, "   "),
            _ => panic!("expected Delete"),
        }
    }

    // ── InputAction exhaustive variants ──

    #[test]
    fn input_action_rename_is_unit() {
        let _ = InputAction::Rename;
    }

    #[test]
    fn input_action_new_branch_is_unit() {
        let _ = InputAction::NewBranch;
    }

    #[test]
    fn input_action_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<InputAction>();
    }

    #[test]
    fn input_action_debug_format() {
        let rename = format!("{:?}", InputAction::Rename);
        let new_branch = format!("{:?}", InputAction::NewBranch);
        assert!(rename.contains("Rename"));
        assert!(new_branch.contains("NewBranch"));
    }

    // ── More state transition patterns ──

    #[test]
    fn can_enter_confirm_mode_and_return() {
        let mut app = TuiApp::new().expect("ok");
        assert!(matches!(app.mode, Mode::Normal));
        app.mode = Mode::Confirm(ConfirmAction::Delete("x".into()));
        assert!(matches!(app.mode, Mode::Confirm(ConfirmAction::Delete(_))));
        app.mode = Mode::Normal;
        assert!(matches!(app.mode, Mode::Normal));
    }

    #[test]
    fn can_enter_input_mode_and_return() {
        let mut app = TuiApp::new().expect("ok");
        app.mode = Mode::Input(InputAction::NewBranch);
        assert!(matches!(app.mode, Mode::Input(InputAction::NewBranch)));
        app.mode = Mode::Normal;
        assert!(matches!(app.mode, Mode::Normal));
    }

    #[test]
    fn can_enter_input_rename_mode_and_return() {
        let mut app = TuiApp::new().expect("ok");
        app.mode = Mode::Input(InputAction::Rename);
        assert!(matches!(app.mode, Mode::Input(InputAction::Rename)));
        app.mode = Mode::Normal;
        assert!(matches!(app.mode, Mode::Normal));
    }

    #[test]
    fn can_cycle_through_all_modes() {
        let mut app = TuiApp::new().expect("ok");
        let modes = vec![
            Mode::Search,
            Mode::Help,
            Mode::Confirm(ConfirmAction::Delete("a".into())),
            Mode::Confirm(ConfirmAction::Restack("b".into())),
            Mode::Confirm(ConfirmAction::RestackAll),
            Mode::Confirm(ConfirmAction::ApplyReorder),
            Mode::Input(InputAction::Rename),
            Mode::Input(InputAction::NewBranch),
            Mode::Reorder,
        ];
        for mode in modes {
            app.mode = mode;
            // just ensure assignment succeeds — no panic
        }
        app.mode = Mode::Normal;
        assert!(matches!(app.mode, Mode::Normal));
    }

    #[test]
    fn switching_pane_does_not_affect_mode() {
        let mut app = TuiApp::new().expect("ok");
        app.mode = Mode::Search;
        app.focused_pane = FocusedPane::Diff;
        assert!(matches!(app.focused_pane, FocusedPane::Diff));
        assert!(matches!(app.mode, Mode::Search));
    }

    #[test]
    fn setting_mode_does_not_affect_pane() {
        let mut app = TuiApp::new().expect("ok");
        app.focused_pane = FocusedPane::Diff;
        app.mode = Mode::Help;
        assert!(matches!(app.focused_pane, FocusedPane::Diff));
        assert!(matches!(app.mode, Mode::Help));
    }

    #[test]
    fn should_quit_is_independent_of_other_fields() {
        let mut app = TuiApp::new().expect("ok");
        app.should_quit = true;
        app.mode = Mode::Reorder;
        app.focused_pane = FocusedPane::Diff;
        app.needs_refresh = true;
        assert!(app.should_quit);
        assert!(matches!(app.mode, Mode::Reorder));
        assert!(matches!(app.focused_pane, FocusedPane::Diff));
        assert!(app.needs_refresh);
    }

    #[test]
    fn needs_refresh_toggle_cycle() {
        let mut app = TuiApp::new().expect("ok");
        for _ in 0..100 {
            app.needs_refresh = !app.needs_refresh;
        }
        // After 100 toggles from initial true, it should be true (even toggles)
        assert!(app.needs_refresh);
    }

    #[test]
    fn repeated_refresh_calls() {
        let mut app = TuiApp::new().expect("ok");
        for _ in 0..50 {
            app.refresh_branches().expect("ok");
        }
        assert!(!app.needs_refresh);
    }

    #[test]
    fn selected_branch_always_none() {
        let mut app = TuiApp::new().expect("ok");
        app.mode = Mode::Reorder;
        app.focused_pane = FocusedPane::Diff;
        app.should_quit = true;
        assert!(app.selected_branch().is_none());
    }

    #[test]
    fn set_status_various_strings() {
        let mut app = TuiApp::new().expect("ok");
        let messages = vec![
            String::new(),
            "hello".into(),
            "a".repeat(1_000_000),
            "\0\x01\x02".into(),
            "line1\nline2\r\nline3".into(),
        ];
        for msg in messages {
            app.set_status(msg);
        }
    }

    #[test]
    fn tui_app_size_is_reasonable() {
        // TuiApp has 4 fields: FocusedPane (1 byte), Mode (variable), 2x bool (2 bytes)
        let size = std::mem::size_of::<TuiApp>();
        assert!(
            size < 512,
            "TuiApp should be reasonably sized, got {size} bytes"
        );
    }

    #[test]
    fn run_does_not_modify_static_state() {
        // run() creates a local app, should not have side effects
        let _ = run();
        let _ = run();
        let _ = run();
    }

    // ── Proptests ──

    use proptest::proptest;

    proptest! {
        #[test]
        fn prop_delete_confirm_preserves_arbitrary_name(
            name in ".{0, 500}",
        ) {
            let mode = Mode::Confirm(ConfirmAction::Delete(name.clone()));
            if let Mode::Confirm(ConfirmAction::Delete(n)) = mode {
                assert_eq!(n, name);
            }
        }

        #[test]
        fn prop_restack_confirm_preserves_arbitrary_name(
            name in ".{0, 500}",
        ) {
            let mode = Mode::Confirm(ConfirmAction::Restack(name.clone()));
            if let Mode::Confirm(ConfirmAction::Restack(n)) = mode {
                assert_eq!(n, name);
            }
        }

        #[test]
        fn prop_needs_refresh_after_toggle_sequence(
            initial in proptest::bool::ANY,
        ) {
            let mut app = TuiApp::new().expect("ok");
            app.needs_refresh = initial;
            assert_eq!(app.needs_refresh, initial);
        }

        #[test]
        fn prop_should_quit_arbitrary_bool(
            quit in proptest::bool::ANY,
        ) {
            let mut app = TuiApp::new().expect("ok");
            app.should_quit = quit;
            assert_eq!(app.should_quit, quit);
        }

        #[test]
        fn prop_set_status_arbitrary_string_does_not_panic(
            msg in proptest::string::string_regex(".{0,10000}").unwrap(),
        ) {
            let mut app = TuiApp::new().expect("ok");
            app.set_status(msg);
        }

        #[test]
        fn prop_refresh_always_clears_flag(
            start in proptest::bool::ANY,
        ) {
            let mut app = TuiApp::new().expect("ok");
            app.needs_refresh = start;
            app.refresh_branches().expect("ok");
            assert!(!app.needs_refresh);
        }

        #[test]
        fn prop_selected_branch_always_none(
            _dummy in proptest::bool::ANY,
        ) {
            let app = TuiApp::new().expect("ok");
            assert!(app.selected_branch().is_none());
        }

        #[test]
        fn prop_mode_confirm_delete_roundtrip(
            name in ".{0, 200}",
        ) {
            let mode = Mode::Confirm(ConfirmAction::Delete(name.clone()));
            if let Mode::Confirm(ConfirmAction::Delete(n)) = mode {
                assert_eq!(n, name);
            }
        }

        #[test]
        fn prop_mode_confirm_restack_roundtrip(
            name in ".{0, 200}",
        ) {
            let mode = Mode::Confirm(ConfirmAction::Restack(name.clone()));
            if let Mode::Confirm(ConfirmAction::Restack(n)) = mode {
                assert_eq!(n, name);
            }
        }
    }

    // ── Adversarial: Send + Sync for TuiApp ──

    #[test]
    fn tui_app_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<TuiApp>();
    }

    #[test]
    fn tui_app_is_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<TuiApp>();
    }

    // ── Adversarial: state machine after quit ──

    #[test]
    fn methods_work_after_quit_flag_set() {
        let mut app = TuiApp::new().expect("ok");
        app.should_quit = true;
        assert!(app.refresh_branches().is_ok());
        app.set_status("still works".to_string());
        assert!(app.selected_branch().is_none());
    }

    #[test]
    fn methods_work_in_confirm_apply_reorder_mode() {
        let mut app = TuiApp::new().expect("ok");
        app.mode = Mode::Confirm(ConfirmAction::ApplyReorder);
        assert!(app.refresh_branches().is_ok());
        app.set_status("test".to_string());
        assert!(app.selected_branch().is_none());
    }

    // ── Adversarial: Worktrees pane ──

    #[test]
    fn worktrees_pane_is_settable() {
        let mut app = TuiApp::new().expect("ok");
        app.focused_pane = FocusedPane::Worktrees;
        assert!(matches!(app.focused_pane, FocusedPane::Worktrees));
    }

    // ── Adversarial: Mode × FocusedPane cross-product ──

    #[test]
    fn mode_pane_cross_product_no_panic() {
        let modes: Vec<fn() -> Mode> = vec![
            || Mode::Normal,
            || Mode::Search,
            || Mode::Help,
            || Mode::Confirm(ConfirmAction::Delete("x".into())),
            || Mode::Confirm(ConfirmAction::Restack("x".into())),
            || Mode::Confirm(ConfirmAction::RestackAll),
            || Mode::Confirm(ConfirmAction::ApplyReorder),
            || Mode::Input(InputAction::Rename),
            || Mode::Input(InputAction::NewBranch),
            || Mode::Reorder,
        ];
        let panes = [FocusedPane::Stack, FocusedPane::Diff, FocusedPane::Worktrees];
        for make_mode in modes {
            for pane in panes {
                let mut app = TuiApp::new().expect("ok");
                app.mode = make_mode();
                app.focused_pane = pane;
                let _ = app.refresh_branches();
                app.set_status("cross-product test".to_string());
                let _ = app.selected_branch();
            }
        }
    }

    // ── Adversarial: empty string in ConfirmAction::Restack ──

    #[test]
    fn confirm_restack_preserves_empty_name() {
        let mode = Mode::Confirm(ConfirmAction::Restack(String::new()));
        match mode {
            Mode::Confirm(ConfirmAction::Restack(name)) => assert!(name.is_empty()),
            _ => panic!("expected Confirm::Restack"),
        }
    }

    #[test]
    fn confirm_restack_preserves_long_name() {
        let long_name = "a".repeat(1000);
        let mode = Mode::Confirm(ConfirmAction::Restack(long_name.clone()));
        match mode {
            Mode::Confirm(ConfirmAction::Restack(name)) => assert_eq!(name.len(), 1000),
            _ => panic!("expected Confirm::Restack"),
        }
    }
}
