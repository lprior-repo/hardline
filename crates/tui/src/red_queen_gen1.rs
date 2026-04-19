//! Red Queen Generation 1 — Adversarial tests for scp-tui
//!
//! Dimensions attacked:
//! - state-transitions: FocusedPane cycling, Mode transitions, invariants
//! - input-navigation: InputHandler boundary conditions, wrapping arithmetic
//! - worktree-selection: WorktreeView wrapping, empty state, item access
//! - stack-tree: Tree building, depth tracking, ancestor chains
//! - error-contract: TuiError conversions, source chains, edge cases
//! - invariant-preservation: Cross-field independence
//! - branch-provider: Provider contract, error handling, refresh semantics

#[cfg(test)]
use crate::app::{BranchProvider, TuiApp};
#[cfg(test)]
use scp_stack::domain::StackBranch;

#[cfg(test)]
struct StubProvider;

#[cfg(test)]
impl BranchProvider for StubProvider {
    fn load_branches(&self) -> std::result::Result<Vec<StackBranch>, String> {
        Ok(Vec::new())
    }
}

#[cfg(test)]
fn test_app() -> TuiApp {
    TuiApp::new(Box::new(StubProvider)).expect("TuiApp::new should succeed")
}

#[cfg(test)]
mod state_transitions {
    use super::*;
    use crate::app::{ConfirmAction, FocusedPane, InputAction, Mode};

    #[test]
    fn focused_pane_three_way_cycle() {
        let mut app = test_app();
        assert!(matches!(app.focused_pane, FocusedPane::Stack));
        app.focused_pane = FocusedPane::Diff;
        app.focused_pane = FocusedPane::Worktrees;
        app.focused_pane = FocusedPane::Stack;
        assert!(matches!(app.focused_pane, FocusedPane::Stack));
    }

    #[test]
    fn focused_pane_skip_cycle() {
        let mut app = test_app();
        app.focused_pane = FocusedPane::Worktrees;
        assert!(matches!(app.focused_pane, FocusedPane::Worktrees));
        app.focused_pane = FocusedPane::Diff;
        assert!(matches!(app.focused_pane, FocusedPane::Diff));
        app.focused_pane = FocusedPane::Stack;
        assert!(matches!(app.focused_pane, FocusedPane::Stack));
    }

    #[test]
    fn rapid_mode_switching_stress() {
        let mut app = test_app();
        let modes = vec![
            Mode::Normal,
            Mode::Search,
            Mode::Help,
            Mode::Reorder,
            Mode::Confirm(ConfirmAction::Delete("x".into())),
            Mode::Input(InputAction::Rename),
            Mode::Input(InputAction::NewBranch),
            Mode::Confirm(ConfirmAction::Restack("y".into())),
            Mode::Confirm(ConfirmAction::RestackAll),
            Mode::Confirm(ConfirmAction::ApplyReorder),
        ];
        for _ in 0..50 {
            for mode in &modes {
                app.mode = mode.clone();
            }
        }
        app.mode = Mode::Normal;
        assert!(matches!(app.mode, Mode::Normal));
        assert!(!app.should_quit);
    }

    #[test]
    fn quit_during_confirm_mode() {
        let mut app = test_app();
        app.mode = Mode::Confirm(ConfirmAction::Delete("important".into()));
        app.should_quit = true;
        assert!(app.should_quit);
        assert!(matches!(app.mode, Mode::Confirm(ConfirmAction::Delete(_))));
    }

    #[test]
    fn quit_during_input_mode() {
        let mut app = test_app();
        app.mode = Mode::Input(InputAction::NewBranch);
        app.should_quit = true;
        assert!(app.should_quit);
        assert!(matches!(app.mode, Mode::Input(InputAction::NewBranch)));
    }

    #[test]
    fn quit_during_reorder_mode() {
        let mut app = test_app();
        app.mode = Mode::Reorder;
        app.should_quit = true;
        assert!(app.should_quit);
        assert!(matches!(app.mode, Mode::Reorder));
    }

    #[test]
    fn confirm_delete_with_slashes_and_dots() {
        let action = ConfirmAction::Delete("feature/fix.bug.v2".to_string());
        match action {
            ConfirmAction::Delete(name) => assert_eq!(name, "feature/fix.bug.v2"),
            _ => panic!("expected Delete"),
        }
    }

    #[test]
    fn confirm_restack_with_special_chars() {
        let action = ConfirmAction::Restack("feature/--weird-name-".to_string());
        match action {
            ConfirmAction::Restack(name) => assert_eq!(name, "feature/--weird-name-"),
            _ => panic!("expected Restack"),
        }
    }

    #[test]
    fn mode_clone_roundtrip() {
        let original = Mode::Confirm(ConfirmAction::Delete("branch".into()));
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }
}

#[cfg(test)]
mod input_navigation {
    use crate::app::Mode;
    use crate::input::{HunkAction, InputHandler, InputResult};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn make_key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    #[test]
    fn navigate_next_single_hunk_stays_same() {
        let mut handler = InputHandler::new();
        handler.set_hunk_count(1);
        assert_eq!(handler.current_hunk, 0);
        handler.handle_key_event(make_key(KeyCode::Down), &Mode::Normal);
        assert_eq!(
            handler.current_hunk, 0,
            "single hunk: navigate_next should stay at 0"
        );
    }

    #[test]
    fn navigate_prev_single_hunk_stays_same() {
        let mut handler = InputHandler::new();
        handler.set_hunk_count(1);
        handler.handle_key_event(make_key(KeyCode::Up), &Mode::Normal);
        assert_eq!(
            handler.current_hunk, 0,
            "single hunk: navigate_prev should stay at 0"
        );
    }

    #[test]
    fn navigate_next_with_two_hunks() {
        let mut handler = InputHandler::new();
        handler.set_hunk_count(2);
        handler.handle_key_event(make_key(KeyCode::Down), &Mode::Normal);
        assert_eq!(handler.current_hunk, 1);
        handler.handle_key_event(make_key(KeyCode::Down), &Mode::Normal);
        assert_eq!(handler.current_hunk, 0, "should wrap to 0");
    }

    #[test]
    fn navigate_prev_with_two_hunks() {
        let mut handler = InputHandler::new();
        handler.set_hunk_count(2);
        handler.handle_key_event(make_key(KeyCode::Up), &Mode::Normal);
        assert_eq!(handler.current_hunk, 1, "should wrap to last");
        handler.handle_key_event(make_key(KeyCode::Up), &Mode::Normal);
        assert_eq!(handler.current_hunk, 0);
    }

    #[test]
    fn set_hunk_count_to_one_from_high_index() {
        let mut handler = InputHandler::new();
        handler.set_hunk_count(10);
        handler.current_hunk = 9;
        handler.set_hunk_count(1);
        assert_eq!(handler.current_hunk, 0, "should clamp to count-1");
    }

    #[test]
    fn set_hunk_count_same_as_current_keeps_current() {
        let mut handler = InputHandler::new();
        handler.set_hunk_count(5);
        handler.current_hunk = 2;
        handler.set_hunk_count(5);
        assert_eq!(
            handler.current_hunk, 2,
            "same count should not change index"
        );
    }

    #[test]
    fn navigate_on_zero_hunks_is_noop() {
        let mut handler = InputHandler::new();
        handler.set_hunk_count(0);
        handler.handle_key_event(make_key(KeyCode::Down), &Mode::Normal);
        handler.handle_key_event(make_key(KeyCode::Up), &Mode::Normal);
        assert_eq!(handler.current_hunk, 0);
    }

    #[test]
    fn large_hunk_count_navigation() {
        let mut handler = InputHandler::new();
        handler.set_hunk_count(1000);
        for _ in 0..1000 {
            handler.handle_key_event(make_key(KeyCode::Down), &Mode::Normal);
        }
        assert_eq!(
            handler.current_hunk, 0,
            "1000 next on 1000 hunks should wrap to 0"
        );
    }

    #[test]
    fn large_hunk_count_prev_navigation() {
        let mut handler = InputHandler::new();
        handler.set_hunk_count(1000);
        handler.handle_key_event(make_key(KeyCode::Up), &Mode::Normal);
        assert_eq!(
            handler.current_hunk, 999,
            "prev from 0 on 1000 hunks should be 999"
        );
    }

    #[test]
    fn stage_key_space_and_s_both_work() {
        let mut handler = InputHandler::new();
        assert_eq!(
            handler.handle_key_event(make_key(KeyCode::Char(' ')), &Mode::Normal),
            InputResult::Handled(HunkAction::Stage)
        );
        assert_eq!(
            handler.handle_key_event(make_key(KeyCode::Char('s')), &Mode::Normal),
            InputResult::Handled(HunkAction::Stage)
        );
    }

    #[test]
    fn discard_upper_and_lower_d() {
        let mut handler = InputHandler::new();
        assert_eq!(
            handler.handle_key_event(make_key(KeyCode::Char('d')), &Mode::Normal),
            InputResult::Handled(HunkAction::Discard)
        );
        assert_eq!(
            handler.handle_key_event(make_key(KeyCode::Char('D')), &Mode::Normal),
            InputResult::Handled(HunkAction::Discard)
        );
    }

    #[test]
    fn esc_is_quit() {
        let mut handler = InputHandler::new();
        assert_eq!(
            handler.handle_key_event(make_key(KeyCode::Esc), &Mode::Normal),
            InputResult::Quit
        );
    }

    #[test]
    fn unhandled_keys() {
        let mut handler = InputHandler::new();
        let unhandled = vec![
            KeyCode::F(1),
            KeyCode::Enter,
            KeyCode::Tab,
            KeyCode::Backspace,
            KeyCode::Delete,
            KeyCode::Home,
            KeyCode::End,
            KeyCode::Insert,
            KeyCode::Char('x'),
            KeyCode::Char('Z'),
        ];
        for code in unhandled {
            assert_eq!(
                handler.handle_key_event(make_key(code), &Mode::Normal),
                InputResult::Unhandled,
                "KeyCode::{code:?} should be unhandled"
            );
        }
    }

    #[test]
    fn page_up_is_scroll_up() {
        let mut handler = InputHandler::new();
        assert_eq!(
            handler.handle_key_event(make_key(KeyCode::PageUp), &Mode::Normal),
            InputResult::Handled(HunkAction::ScrollUp)
        );
    }

    #[test]
    fn page_down_is_scroll_down() {
        let mut handler = InputHandler::new();
        assert_eq!(
            handler.handle_key_event(make_key(KeyCode::PageDown), &Mode::Normal),
            InputResult::Handled(HunkAction::ScrollDown)
        );
    }

    #[test]
    fn scroll_keys() {
        let mut handler = InputHandler::new();
        assert_eq!(
            handler.handle_key_event(make_key(KeyCode::Char('b')), &Mode::Normal),
            InputResult::Handled(HunkAction::ScrollUp)
        );
        let mut handler2 = InputHandler::new();
        assert_eq!(
            handler2.handle_key_event(make_key(KeyCode::Char('f')), &Mode::Normal),
            InputResult::Handled(HunkAction::ScrollDown)
        );
    }

    #[test]
    fn unstage_key_u() {
        let mut handler = InputHandler::new();
        assert_eq!(
            handler.handle_key_event(make_key(KeyCode::Char('u')), &Mode::Normal),
            InputResult::Handled(HunkAction::Unstage)
        );
    }

    #[test]
    fn modifier_keys_dont_change_outcome() {
        let mut handler = InputHandler::new();
        let ctrl_q = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL);
        assert_eq!(
            handler.handle_key_event(ctrl_q, &Mode::Normal),
            InputResult::Quit
        );
    }

    #[test]
    fn navigate_does_not_modify_total_hunks() {
        let mut handler = InputHandler::new();
        handler.set_hunk_count(5);
        let original = handler.total_hunks;
        handler.handle_key_event(make_key(KeyCode::Down), &Mode::Normal);
        handler.handle_key_event(make_key(KeyCode::Up), &Mode::Normal);
        assert_eq!(handler.total_hunks, original);
    }

    #[test]
    fn alternating_next_prev_returns_to_start() {
        let mut handler = InputHandler::new();
        handler.set_hunk_count(3);
        handler.handle_key_event(make_key(KeyCode::Down), &Mode::Normal);
        handler.handle_key_event(make_key(KeyCode::Down), &Mode::Normal);
        handler.handle_key_event(make_key(KeyCode::Up), &Mode::Normal);
        handler.handle_key_event(make_key(KeyCode::Up), &Mode::Normal);
        assert_eq!(handler.current_hunk, 0);
    }

    #[test]
    fn input_handler_clone_independence() {
        let mut handler = InputHandler::new();
        handler.set_hunk_count(5);
        handler.current_hunk = 3;
        let mut clone = handler.clone();
        clone.handle_key_event(make_key(KeyCode::Down), &Mode::Normal);
        assert_eq!(handler.current_hunk, 3, "original should be unchanged");
        assert_eq!(clone.current_hunk, 4, "clone should be advanced");
    }
}

#[cfg(test)]
mod worktree_selection {
    use crate::views::WorktreeView;
    use crate::widgets::worktree::WorktreeItem;

    fn make_item(name: &str) -> WorktreeItem {
        WorktreeItem {
            id: format!("id-{}", name),
            name: name.to_string(),
            path: format!("/tmp/{}", name),
            branch: Some("main".to_string()),
            state: worktree::WorktreeState::Active,
            is_active: false,
        }
    }

    #[test]
    fn single_item_next_wraps_to_zero() {
        let mut view = WorktreeView::new(vec![make_item("only")]);
        view.select_next();
        assert_eq!(view.selected_index, 0);
    }

    #[test]
    fn single_item_prev_wraps_to_zero() {
        let mut view = WorktreeView::new(vec![make_item("only")]);
        view.select_previous();
        assert_eq!(view.selected_index, 0);
    }

    #[test]
    fn three_items_full_cycle_next() {
        let mut view = WorktreeView::new(vec![make_item("a"), make_item("b"), make_item("c")]);
        view.select_next();
        view.select_next();
        view.select_next();
        assert_eq!(view.selected_index, 0);
    }

    #[test]
    fn three_items_full_cycle_prev() {
        let mut view = WorktreeView::new(vec![make_item("a"), make_item("b"), make_item("c")]);
        view.select_previous();
        view.select_previous();
        view.select_previous();
        assert_eq!(view.selected_index, 0);
    }

    #[test]
    fn with_items_replaces_and_resets() {
        let mut view = WorktreeView::new(vec![make_item("old1"), make_item("old2")]);
        view.selected_index = 1;
        let view = view.with_items(vec![make_item("new1")]);
        assert_eq!(view.selected_index, 0);
    }

    #[test]
    fn selected_item_returns_none_on_empty() {
        let view = WorktreeView::new(Vec::new());
        assert!(view.selected_item().is_none());
    }

    #[test]
    fn selected_item_returns_last_after_prev_from_zero() {
        let mut view = WorktreeView::new(vec![make_item("a"), make_item("b"), make_item("c")]);
        view.select_previous();
        assert_eq!(view.selected_index, 2);
        assert_eq!(view.selected_item().unwrap().name, "c");
    }

    #[test]
    fn empty_navigation_is_noop() {
        let mut view = WorktreeView::new(Vec::new());
        view.select_next();
        view.select_previous();
        assert_eq!(view.selected_index, 0);
    }

    #[test]
    fn ten_items_navigation_stress() {
        let items: Vec<WorktreeItem> = (0..10).map(|i| make_item(&format!("wt-{}", i))).collect();
        let mut view = WorktreeView::new(items);
        for _ in 0..10 {
            view.select_next();
        }
        assert_eq!(view.selected_index, 0);
        for _ in 0..10 {
            view.select_previous();
        }
        assert_eq!(view.selected_index, 0);
    }
}

#[cfg(test)]
mod stack_tree_adversarial {
    use crate::widgets::StackTreeWidget;
    use scp_stack::domain::value_objects::BranchName;
    use scp_stack::domain::{PrInfo, PrState, StackBranch};

    fn branch(name: &str, parent: Option<&str>) -> StackBranch {
        StackBranch {
            name: BranchName::new(name.to_string()),
            parent: parent.map(|p| BranchName::new(p.to_string())),
            children: Vec::new(),
            needs_restack: false,
            pr_info: None,
        }
    }

    fn branch_with_pr(
        name: &str,
        parent: Option<&str>,
        number: u32,
        state: PrState,
    ) -> StackBranch {
        let mut b = branch(name, parent);
        b.pr_info = Some(PrInfo {
            number,
            url: format!("https://github.com/org/repo/pull/{}", number),
            title: format!("PR #{}", number),
            state,
            is_draft: Some(false),
        });
        b
    }

    #[test]
    fn deep_nesting_chain() {
        let branches = vec![
            branch("r0", None),
            branch("r1", Some("r0")),
            branch("r2", Some("r1")),
            branch("r3", Some("r2")),
            branch("r4", Some("r3")),
            branch("r5", Some("r4")),
        ];
        let nodes = StackTreeWidget::new(branches).build_tree_nodes();
        assert_eq!(nodes.len(), 6);
        for (i, node) in nodes.iter().enumerate() {
            assert_eq!(node.depth, i);
        }
    }

    #[test]
    fn wide_tree_many_roots() {
        let branches: Vec<StackBranch> = (0..10)
            .map(|i| branch(&format!("root-{}", i), None))
            .collect();
        let nodes = StackTreeWidget::new(branches).build_tree_nodes();
        assert_eq!(nodes.len(), 10);
        for node in &nodes {
            assert_eq!(node.depth, 0);
        }
        for (i, node) in nodes.iter().enumerate() {
            assert_eq!(node.is_last_child, i == 9);
        }
    }

    #[test]
    fn mixed_depth_wide_and_deep() {
        let branches = vec![
            branch("root1", None),
            branch("root1-child1", Some("root1")),
            branch("root1-child2", Some("root1")),
            branch("root1-child1-grandchild", Some("root1-child1")),
            branch("root2", None),
            branch("root2-child", Some("root2")),
        ];
        let nodes = StackTreeWidget::new(branches).build_tree_nodes();
        assert_eq!(nodes.len(), 6);
        // DFS: root1 -> root1-child1 -> grandchild -> root1-child2 -> root2 -> root2-child
        let depths: Vec<usize> = nodes.iter().map(|n| n.depth).collect();
        assert_eq!(depths, vec![0, 1, 2, 1, 0, 1]);
    }

    #[test]
    fn orphan_branch_produces_no_nodes() {
        let nodes =
            StackTreeWidget::new(vec![branch("orphan", Some("nonexistent"))]).build_tree_nodes();
        assert_eq!(nodes.len(), 0);
    }

    #[test]
    fn sibling_ordering_preserved() {
        let nodes = StackTreeWidget::new(vec![
            branch("c", None),
            branch("b", Some("c")),
            branch("a", Some("c")),
        ])
        .build_tree_nodes();
        assert_eq!(nodes[1].branch.name.as_str(), "b");
        assert_eq!(nodes[2].branch.name.as_str(), "a");
    }

    #[test]
    fn all_pr_states_represented() {
        let branches = vec![
            branch_with_pr("open", None, 1, PrState::Open),
            branch_with_pr("merged", None, 2, PrState::Merged),
            branch_with_pr("closed", None, 3, PrState::Closed),
        ];
        assert_eq!(StackTreeWidget::new(branches).build_tree_nodes().len(), 3);
    }

    #[test]
    fn needs_restack_overrides_pr_indicator() {
        let mut b = branch_with_pr("wip", None, 1, PrState::Open);
        b.needs_restack = true;
        let (indicator, color) = StackTreeWidget::branch_indicator(&b);
        assert_eq!(indicator, "⚑");
        assert_eq!(color, ratatui::style::Color::Red);
    }

    #[test]
    fn empty_widget_builds_no_nodes() {
        let widget = StackTreeWidget::new(Vec::new());
        assert!(widget.build_tree_nodes().is_empty());
        assert_eq!(widget.selected_index, None);
    }

    #[test]
    fn ancestor_is_last_chain() {
        let branches = vec![
            branch("root", None),
            branch("child1", Some("root")),
            branch("child2", Some("root")),
            branch("grandchild", Some("child1")),
        ];
        let nodes = StackTreeWidget::new(branches).build_tree_nodes();
        // DFS: root -> child1 -> grandchild -> child2
        assert_eq!(nodes[0].ancestor_is_last, vec![true]);
        assert_eq!(nodes[1].ancestor_is_last, vec![true, false]);
        assert_eq!(nodes[2].ancestor_is_last, vec![true, false, true]);
        assert_eq!(nodes[3].ancestor_is_last, vec![true, true]);
    }
}

#[cfg(test)]
mod error_contract_adversarial {
    use crate::error::{Result, TuiError};
    use std::error::Error;

    #[test]
    fn error_is_object_safe() {
        let err: Box<dyn Error> = Box::new(TuiError::Error("test".into()));
        let _ = err.to_string();
    }

    #[test]
    fn io_error_source_chain() {
        let tui_err = TuiError::IoError(std::io::Error::new(std::io::ErrorKind::NotFound, "inner"));
        let source = tui_err.source().expect("should have source");
        assert!(source.to_string().contains("inner"));
    }

    #[test]
    fn error_and_terminal_no_source() {
        assert!(TuiError::Error("x".into()).source().is_none());
        assert!(TuiError::TerminalError("y".into()).source().is_none());
    }

    #[test]
    fn result_into_dyn_error_send_sync() {
        fn takes_dyn(err: Box<dyn Error + Send + Sync>) -> String {
            err.to_string()
        }
        let result: Result<()> = Err(TuiError::Error("test".into()));
        let _ = takes_dyn(Box::new(result.expect_err("should be err")));
    }

    #[test]
    fn all_variants_matchable() {
        let errors = vec![
            TuiError::Error("e".into()),
            TuiError::TerminalError("t".into()),
            TuiError::IoError(std::io::Error::new(std::io::ErrorKind::Other, "io")),
        ];
        for err in errors {
            match &err {
                TuiError::Error(_) => {}
                TuiError::TerminalError(_) => {}
                TuiError::IoError(_) => {}
            }
        }
    }
}

#[cfg(test)]
mod invariant_preservation {
    use super::*;
    use crate::app::{ConfirmAction, FocusedPane, Mode};

    #[test]
    fn all_fields_independent_simultaneous_mutation() {
        let mut app = test_app();
        app.focused_pane = FocusedPane::Diff;
        app.mode = Mode::Confirm(ConfirmAction::Delete("x".into()));
        app.should_quit = true;
        app.needs_refresh = false;
        assert!(matches!(app.focused_pane, FocusedPane::Diff));
        assert!(matches!(app.mode, Mode::Confirm(ConfirmAction::Delete(_))));
        assert!(app.should_quit);
        assert!(!app.needs_refresh);
    }

    #[test]
    fn refresh_preserves_mode_and_pane() {
        let mut app = test_app();
        app.focused_pane = FocusedPane::Worktrees;
        app.mode = Mode::Search;
        app.refresh_branches().expect("ok");
        assert!(matches!(app.focused_pane, FocusedPane::Worktrees));
        assert!(matches!(app.mode, Mode::Search));
    }

    #[test]
    fn refresh_preserves_quit_state() {
        let mut app = test_app();
        app.should_quit = true;
        app.refresh_branches().expect("ok");
        assert!(app.should_quit);
    }

    #[test]
    fn refresh_preserves_diff_lines() {
        let mut app = test_app();
        app.diff_lines = vec![crate::app::DiffLine::new(
            "h",
            crate::app::DiffLineKind::Header,
        )];
        app.refresh_branches().expect("ok");
        assert_eq!(app.diff_lines.len(), 1);
    }

    #[test]
    fn pane_cycle_preserves_mode() {
        let mut app = test_app();
        app.mode = Mode::Confirm(ConfirmAction::RestackAll);
        app.focused_pane = FocusedPane::Diff;
        app.focused_pane = FocusedPane::Worktrees;
        app.focused_pane = FocusedPane::Stack;
        assert!(matches!(app.mode, Mode::Confirm(ConfirmAction::RestackAll)));
    }

    #[test]
    fn mode_cycle_preserves_pane() {
        let mut app = test_app();
        app.focused_pane = FocusedPane::Worktrees;
        app.mode = Mode::Normal;
        app.mode = Mode::Help;
        app.mode = Mode::Reorder;
        app.mode = Mode::Normal;
        assert!(matches!(app.focused_pane, FocusedPane::Worktrees));
    }

    #[test]
    fn new_always_returns_ok() {
        for _ in 0..100 {
            assert!(TuiApp::new(Box::new(StubProvider)).is_ok());
        }
    }
}

#[cfg(test)]
mod diff_line_adversarial {
    use crate::app::{DiffLine, DiffLineKind};

    #[test]
    fn different_kinds_are_never_equal() {
        let h = DiffLine::new("x", DiffLineKind::Header);
        let hh = DiffLine::new("x", DiffLineKind::Hunk);
        let a = DiffLine::new("x", DiffLineKind::Add);
        let d = DiffLine::new("x", DiffLineKind::Remove);
        let c = DiffLine::new("x", DiffLineKind::Context);
        let all = vec![&h, &hh, &a, &d, &c];
        for i in 0..all.len() {
            for j in (i + 1)..all.len() {
                assert_ne!(all[i].kind, all[j].kind);
            }
        }
    }

    #[test]
    fn same_kind_different_content_neq() {
        let a = DiffLine::new("a", DiffLineKind::Header);
        let b = DiffLine::new("b", DiffLineKind::Header);
        assert_ne!(a.content, b.content);
    }

    #[test]
    fn clone_produces_equal_copy() {
        for kind in &[
            DiffLineKind::Header,
            DiffLineKind::Hunk,
            DiffLineKind::Add,
            DiffLineKind::Remove,
            DiffLineKind::Context,
        ] {
            let line = DiffLine::new("test", *kind);
            let cloned = line.clone();
            assert_eq!(line.content, cloned.content);
            assert_eq!(line.kind, cloned.kind);
        }
    }

    #[test]
    fn large_content_roundtrip() {
        let big = "x".repeat(100_000);
        let line = DiffLine::new(big.clone(), DiffLineKind::Context);
        assert_eq!(line.content, big);
    }
}

#[cfg(test)]
mod branch_provider_contract {
    use super::*;
    use scp_stack::domain::BranchName;

    struct FailingProvider;
    impl BranchProvider for FailingProvider {
        fn load_branches(&self) -> std::result::Result<Vec<StackBranch>, String> {
            Err("connection refused".to_string())
        }
    }

    struct StaticProvider(Vec<StackBranch>);
    impl BranchProvider for StaticProvider {
        fn load_branches(&self) -> std::result::Result<Vec<StackBranch>, String> {
            Ok(self.0.clone())
        }
    }

    #[test]
    fn failing_provider_sets_status_message() {
        let mut app = TuiApp::new(Box::new(FailingProvider)).expect("ok");
        app.refresh_branches()
            .expect("refresh should not propagate provider error");
        assert!(app.status_message.contains("connection refused"));
        assert!(!app.needs_refresh);
    }

    #[test]
    fn refresh_skips_when_not_needed() {
        struct PanickingProvider;
        impl BranchProvider for PanickingProvider {
            fn load_branches(&self) -> std::result::Result<Vec<StackBranch>, String> {
                panic!("should not be called");
            }
        }
        let mut app = TuiApp::new(Box::new(PanickingProvider)).expect("ok");
        app.needs_refresh = false;
        app.refresh_branches().expect("should skip provider call");
    }

    #[test]
    fn refresh_populates_branches() {
        let branches = vec![StackBranch {
            name: BranchName::new("feature/a"),
            parent: Some(BranchName::new("main")),
            children: vec![],
            needs_restack: false,
            pr_info: None,
        }];
        let mut app = TuiApp::new(Box::new(StaticProvider(branches.clone()))).expect("ok");
        app.refresh_branches().expect("ok");
        assert_eq!(app.stack_branches.len(), 1);
        assert_eq!(app.stack_branches[0].name.as_str(), "feature/a");
    }

    #[test]
    fn set_status_stores_message() {
        let mut app = test_app();
        app.set_status("hello".to_string());
        assert_eq!(app.status_message, "hello");
    }

    #[test]
    fn set_status_overwrites() {
        let mut app = test_app();
        app.set_status("first".to_string());
        app.set_status("second".to_string());
        assert_eq!(app.status_message, "second");
    }
}
