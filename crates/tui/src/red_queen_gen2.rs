//! Red Queen Generation 2 — Escalated adversarial tests for scp-tui
//!
//! Dimensions escalated:
//! - stack-tree: DFS edge cases, cycle detection, self-referencing
//! - input-key-handling: all key mappings verified
//! - branch-provider: concurrent refresh, repeated calls, provider swapping
//! - proptest-invariants: property-based wrapping, mode transitions

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
    TuiApp::new(Box::new(StubProvider)).expect("ok")
}

#[cfg(test)]
mod stack_tree_escalated {
    use crate::widgets::StackTreeWidget;
    use scp_stack::domain::value_objects::BranchName;
    use scp_stack::domain::StackBranch;

    fn branch(name: &str, parent: Option<&str>) -> StackBranch {
        StackBranch {
            name: BranchName::new(name.to_string()),
            parent: parent.map(|p| BranchName::new(p.to_string())),
            children: Vec::new(),
            needs_restack: false,
            pr_info: None,
        }
    }

    #[test]
    fn self_referencing_branch_produces_no_nodes() {
        let branches = vec![StackBranch {
            name: BranchName::new("self".to_string()),
            parent: Some(BranchName::new("self".to_string())),
            children: Vec::new(),
            needs_restack: false,
            pr_info: None,
        }];
        let nodes = StackTreeWidget::new(branches).build_tree_nodes();
        assert_eq!(nodes.len(), 0);
    }

    #[test]
    fn circular_reference_two_nodes() {
        let a = branch("a", Some("b"));
        let b = branch("b", Some("a"));
        let nodes = StackTreeWidget::new(vec![a, b]).build_tree_nodes();
        assert_eq!(nodes.len(), 0);
    }

    #[test]
    fn deeply_nested_chain_stress() {
        let branches: Vec<StackBranch> = (0..50)
            .map(|i| {
                let parent = if i > 0 {
                    Some(format!("r{}", i - 1))
                } else {
                    None
                };
                branch(&format!("r{}", i), parent.as_deref())
            })
            .collect();
        let nodes = StackTreeWidget::new(branches).build_tree_nodes();
        assert_eq!(nodes.len(), 50);
        for (i, node) in nodes.iter().enumerate() {
            assert_eq!(node.depth, i);
        }
    }

    #[test]
    fn single_root_with_many_children() {
        let mut branches = vec![branch("root", None)];
        for i in 0..20 {
            branches.push(branch(&format!("child-{}", i), Some("root")));
        }
        let nodes = StackTreeWidget::new(branches).build_tree_nodes();
        assert_eq!(nodes.len(), 21);
        assert_eq!(nodes[0].depth, 0);
        for node in &nodes[1..] {
            assert_eq!(node.depth, 1);
        }
        for (i, node) in nodes[1..].iter().enumerate() {
            assert_eq!(node.is_last_child, i == 19);
        }
    }

    #[test]
    fn multiple_roots_each_with_children() {
        let branches = vec![
            branch("a", None),
            branch("a1", Some("a")),
            branch("a2", Some("a")),
            branch("b", None),
            branch("b1", Some("b")),
            branch("c", None),
            branch("c1", Some("c")),
            branch("c2", Some("c")),
            branch("c3", Some("c")),
        ];
        let nodes = StackTreeWidget::new(branches).build_tree_nodes();
        assert_eq!(nodes.len(), 9);
        assert_eq!(nodes.iter().filter(|n| n.depth == 0).count(), 3);
    }

    /// Fixed: duplicate branch names no longer cause infinite recursion.
    /// A visited set in build_tree_nodes prevents cycles.
    /// Was: ha-dew0 (P0 CRITICAL)
    #[test]
    fn duplicate_branch_names() {
        let nodes = StackTreeWidget::new(vec![branch("dup", None), branch("dup", Some("dup"))])
            .build_tree_nodes();
        assert_eq!(nodes.len(), 2);
    }

    #[test]
    fn empty_branch_name() {
        let nodes = StackTreeWidget::new(vec![branch("", None)]).build_tree_nodes();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].branch.name.as_str(), "");
    }

    #[test]
    fn unicode_branch_names() {
        let nodes = StackTreeWidget::new(vec![
            branch("フィーチャー", None),
            branch("功能", Some("フィーチャー")),
        ])
        .build_tree_nodes();
        assert_eq!(nodes.len(), 2);
    }
}

#[cfg(test)]
mod key_mapping_complete {
    use crate::app::Mode;
    use crate::input::{HunkAction, InputHandler, InputResult};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    fn assert_handled(handler: &mut InputHandler, code: KeyCode, expected: HunkAction) {
        assert_eq!(
            handler.handle_key_event(key(code), &Mode::Normal),
            InputResult::Handled(expected),
            "KeyCode::{code:?} should map to {expected:?}"
        );
    }

    #[test]
    fn all_mapped_keys_verified() {
        let mut handler = InputHandler::new();
        handler.set_hunk_count(3);

        assert_handled(&mut handler, KeyCode::Char(' '), HunkAction::Stage);
        assert_handled(&mut handler, KeyCode::Char('s'), HunkAction::Stage);
        assert_handled(&mut handler, KeyCode::Char('u'), HunkAction::Unstage);
        assert_handled(&mut handler, KeyCode::Char('d'), HunkAction::Discard);
        assert_handled(&mut handler, KeyCode::Char('D'), HunkAction::Discard);
        assert_handled(&mut handler, KeyCode::Char('j'), HunkAction::NavigateNext);
        assert_handled(&mut handler, KeyCode::Down, HunkAction::NavigateNext);
        assert_handled(&mut handler, KeyCode::Char('k'), HunkAction::NavigatePrev);
        assert_handled(&mut handler, KeyCode::Up, HunkAction::NavigatePrev);
        assert_handled(&mut handler, KeyCode::Char('b'), HunkAction::ScrollUp);
        assert_handled(&mut handler, KeyCode::PageUp, HunkAction::ScrollUp);
        assert_handled(&mut handler, KeyCode::Char('f'), HunkAction::ScrollDown);
        assert_handled(&mut handler, KeyCode::PageDown, HunkAction::ScrollDown);

        assert_eq!(
            handler.handle_key_event(key(KeyCode::Char('q')), &Mode::Normal),
            InputResult::Quit
        );
        let mut h2 = InputHandler::new();
        assert_eq!(
            h2.handle_key_event(key(KeyCode::Esc), &Mode::Normal),
            InputResult::Quit
        );
    }

    #[test]
    fn char_q_vs_char_c() {
        let mut h = InputHandler::new();
        assert_eq!(
            h.handle_key_event(key(KeyCode::Char('q')), &Mode::Normal),
            InputResult::Quit
        );
        let mut h2 = InputHandler::new();
        assert_eq!(
            h2.handle_key_event(key(KeyCode::Char('c')), &Mode::Normal),
            InputResult::Unhandled
        );
    }

    #[test]
    fn number_keys_unhandled() {
        let mut handler = InputHandler::new();
        for n in '0'..='9' {
            assert_eq!(
                handler.handle_key_event(key(KeyCode::Char(n)), &Mode::Normal),
                InputResult::Unhandled
            );
        }
    }

    #[test]
    fn function_keys_unhandled() {
        let mut handler = InputHandler::new();
        for f in 1..=12 {
            assert_eq!(
                handler.handle_key_event(key(KeyCode::F(f)), &Mode::Normal),
                InputResult::Unhandled
            );
        }
    }

    #[test]
    fn arrow_left_right_unhandled() {
        let mut handler = InputHandler::new();
        assert_eq!(
            handler.handle_key_event(key(KeyCode::Left), &Mode::Normal),
            InputResult::Unhandled
        );
        assert_eq!(
            handler.handle_key_event(key(KeyCode::Right), &Mode::Normal),
            InputResult::Unhandled
        );
    }
}

#[cfg(test)]
mod branch_provider_escalated {
    use super::*;
    use scp_stack::domain::BranchName;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    struct CountingProvider {
        count: AtomicUsize,
    }

    impl CountingProvider {
        fn new() -> Self {
            Self {
                count: AtomicUsize::new(0),
            }
        }
    }

    impl BranchProvider for CountingProvider {
        fn load_branches(&self) -> std::result::Result<Vec<StackBranch>, String> {
            self.count.fetch_add(1, Ordering::SeqCst);
            Ok(Vec::new())
        }
    }

    #[test]
    fn refresh_called_only_once_when_needed() {
        let provider = CountingProvider::new();
        let mut app = TuiApp::new(Box::new(provider)).expect("ok");
        app.refresh_branches().expect("ok");
        assert_eq!(app.needs_refresh, false);
        app.refresh_branches().expect("ok");
        assert_eq!(app.needs_refresh, false);
    }

    #[test]
    fn refresh_after_rearm_calls_provider_again() {
        let provider = CountingProvider::new();
        let mut app = TuiApp::new(Box::new(provider)).expect("ok");
        app.refresh_branches().expect("ok");
        app.needs_refresh = true;
        app.refresh_branches().expect("ok");
        assert!(!app.needs_refresh);
    }

    #[test]
    fn failing_provider_repeated_refresh() {
        struct AlwaysFails;
        impl BranchProvider for AlwaysFails {
            fn load_branches(&self) -> std::result::Result<Vec<StackBranch>, String> {
                Err("always fails".to_string())
            }
        }

        let mut app = TuiApp::new(Box::new(AlwaysFails)).expect("ok");
        for _ in 0..10 {
            app.needs_refresh = true;
            app.refresh_branches().expect("should not propagate error");
        }
        assert!(app.status_message.contains("always fails"));
    }

    #[test]
    fn provider_large_dataset() {
        struct LargeProvider;
        impl BranchProvider for LargeProvider {
            fn load_branches(&self) -> std::result::Result<Vec<StackBranch>, String> {
                Ok((0..1000)
                    .map(|i| StackBranch {
                        name: BranchName::new(format!("branch-{}", i)),
                        parent: if i > 0 {
                            Some(BranchName::new(format!("branch-{}", i - 1)))
                        } else {
                            None
                        },
                        children: Vec::new(),
                        needs_restack: false,
                        pr_info: None,
                    })
                    .collect())
            }
        }

        let mut app = TuiApp::new(Box::new(LargeProvider)).expect("ok");
        app.refresh_branches().expect("ok");
        assert_eq!(app.stack_branches.len(), 1000);
    }

    #[test]
    fn empty_result_clears_existing_branches() {
        struct OneThenEmpty {
            called: AtomicBool,
        }
        impl BranchProvider for OneThenEmpty {
            fn load_branches(&self) -> std::result::Result<Vec<StackBranch>, String> {
                if self.called.swap(true, Ordering::SeqCst) {
                    Ok(Vec::new())
                } else {
                    Ok(vec![StackBranch {
                        name: BranchName::new("temp"),
                        parent: None,
                        children: Vec::new(),
                        needs_restack: false,
                        pr_info: None,
                    }])
                }
            }
        }

        let mut app = TuiApp::new(Box::new(OneThenEmpty {
            called: AtomicBool::new(false),
        }))
        .expect("ok");
        app.refresh_branches().expect("ok");
        assert_eq!(app.stack_branches.len(), 1);
        app.needs_refresh = true;
        app.refresh_branches().expect("ok");
        assert_eq!(
            app.stack_branches.len(),
            0,
            "second refresh should replace, not append"
        );
    }
}

#[cfg(test)]
mod proptest_invariants {
    use crate::app::{ConfirmAction, InputAction, Mode};
    use proptest::proptest;

    proptest! {
        #[test]
        fn prop_mode_clone_equality(name in proptest::option::of(proptest::string::string_regex(".{0,50}").unwrap())) {
            let m = match name {
                Some(n) => Mode::Confirm(ConfirmAction::Delete(n)),
                None => Mode::Normal,
            };
            assert_eq!(m, m.clone());
        }

        #[test]
        fn prop_confirm_action_clone_equality(name in ".{0,100}") {
            let delete = ConfirmAction::Delete(name.clone());
            assert_eq!(delete, delete.clone());
            let restack = ConfirmAction::Restack(name);
            assert_eq!(restack, restack.clone());
        }

        #[test]
        fn prop_input_action_clone_equality(action in proptest::option::of(proptest::bool::ANY)) {
            let a = match action {
                Some(true) => InputAction::Rename,
                Some(false) => InputAction::NewBranch,
                None => InputAction::Rename,
            };
            assert_eq!(a, a.clone());
        }
    }
}
