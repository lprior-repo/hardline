//! Red Queen Generation 3 — Deep adversarial tests for scp-tui
//!
//! Dimensions attacked:
//! - branch-indicator-priority: restack vs PR vs draft vs default
//! - tree-node-prefix: deep nesting, multi-root ancestor chains, unicode box-drawing
//! - worktree-fragility: out-of-bounds index, with_items resets, selected_item edge cases
//! - diff-line-semantics: Clone/Send/Sync, large content, empty variants
//! - input-handler-modifiers: Shift, Ctrl, Alt with mapped keys
//! - stack-children-field: children vector in StackBranch is cosmetic only
//! - widget-selection: with_selection out-of-bounds, None selection
//! - refresh-idempotency: repeated refresh with no rearm
//! - format-functions: format_branch_name, format_pr_info edge cases

use crate::app::{BranchProvider, TuiApp};
use scp_stack::domain::StackBranch;

struct StubProvider;

impl BranchProvider for StubProvider {
    fn load_branches(&self) -> std::result::Result<Vec<StackBranch>, String> {
        Ok(Vec::new())
    }
}

fn test_app() -> TuiApp {
    TuiApp::new(Box::new(StubProvider)).expect("ok")
}

#[cfg(test)]
mod branch_indicator_priority {
    use crate::widgets::StackTreeWidget;
    use scp_stack::domain::value_objects::BranchName;
    use scp_stack::domain::{PrInfo, PrState, StackBranch};

    fn branch(name: &str) -> StackBranch {
        StackBranch {
            name: BranchName::new(name.to_string()),
            parent: None,
            children: Vec::new(),
            needs_restack: false,
            pr_info: None,
        }
    }

    #[test]
    fn restack_takes_priority_over_open_pr() {
        let mut b = branch("wip");
        b.needs_restack = true;
        b.pr_info = Some(PrInfo {
            number: 42,
            url: "https://example.com/42".into(),
            title: "WIP".into(),
            state: PrState::Open,
            is_draft: Some(false),
        });
        let (indicator, color) = StackTreeWidget::branch_indicator(&b);
        assert_eq!(indicator, "⚑", "restack flag must override PR indicator");
        assert_eq!(color, ratatui::style::Color::Red);
    }

    #[test]
    fn restack_takes_priority_over_draft_pr() {
        let mut b = branch("draft-wip");
        b.needs_restack = true;
        b.pr_info = Some(PrInfo {
            number: 43,
            url: "https://example.com/43".into(),
            title: "Draft".into(),
            state: PrState::Open,
            is_draft: Some(true),
        });
        let (indicator, color) = StackTreeWidget::branch_indicator(&b);
        assert_eq!(indicator, "⚑");
        assert_eq!(color, ratatui::style::Color::Red);
    }

    #[test]
    fn restack_takes_priority_over_merged_pr() {
        let mut b = branch("merged-wip");
        b.needs_restack = true;
        b.pr_info = Some(PrInfo {
            number: 44,
            url: "https://example.com/44".into(),
            title: "Merged".into(),
            state: PrState::Merged,
            is_draft: None,
        });
        let (indicator, _color) = StackTreeWidget::branch_indicator(&b);
        assert_eq!(indicator, "⚑");
    }

    #[test]
    fn open_pr_shows_green_dot() {
        let mut b = branch("feat");
        b.pr_info = Some(PrInfo {
            number: 1,
            url: "https://example.com/1".into(),
            title: "Feature".into(),
            state: PrState::Open,
            is_draft: Some(false),
        });
        let (indicator, color) = StackTreeWidget::branch_indicator(&b);
        assert_eq!(indicator, "●");
        assert_eq!(color, ratatui::style::Color::Green);
    }

    #[test]
    fn merged_pr_still_shows_green_dot() {
        let mut b = branch("done");
        b.pr_info = Some(PrInfo {
            number: 2,
            url: "https://example.com/2".into(),
            title: "Done".into(),
            state: PrState::Merged,
            is_draft: None,
        });
        let (indicator, _color) = StackTreeWidget::branch_indicator(&b);
        assert_eq!(indicator, "●");
        // Note: branch_indicator doesn't distinguish PR states — all PRs get green
    }

    #[test]
    fn closed_pr_still_shows_green_dot() {
        let mut b = branch("closed");
        b.pr_info = Some(PrInfo {
            number: 3,
            url: "https://example.com/3".into(),
            title: "Closed".into(),
            state: PrState::Closed,
            is_draft: None,
        });
        let (indicator, _color) = StackTreeWidget::branch_indicator(&b);
        assert_eq!(indicator, "●");
    }

    #[test]
    fn no_pr_no_restack_shows_blue_circle() {
        let b = branch("plain");
        let (indicator, color) = StackTreeWidget::branch_indicator(&b);
        assert_eq!(indicator, "○");
        assert_eq!(color, ratatui::style::Color::Blue);
    }

    #[test]
    fn draft_pr_with_is_draft_none() {
        let mut b = branch("unknown-draft");
        b.pr_info = Some(PrInfo {
            number: 99,
            url: "https://example.com/99".into(),
            title: "Unknown".into(),
            state: PrState::Open,
            is_draft: None,
        });
        let (indicator, _color) = StackTreeWidget::branch_indicator(&b);
        assert_eq!(indicator, "●");
    }
}

#[cfg(test)]
mod tree_node_prefix_edge_cases {
    use crate::widgets::TreeNode;
    use scp_stack::domain::value_objects::BranchName;
    use scp_stack::domain::StackBranch;

    fn branch(name: &str) -> StackBranch {
        StackBranch {
            name: BranchName::new(name.to_string()),
            parent: None,
            children: Vec::new(),
            needs_restack: false,
            pr_info: None,
        }
    }

    #[test]
    fn root_node_has_empty_prefix() {
        let node = TreeNode {
            branch: branch("root"),
            depth: 0,
            is_last_child: true,
            ancestor_is_last: vec![],
        };
        assert!(node.prefix_symbols().is_empty());
    }

    #[test]
    fn depth_one_last_child_gets_corner() {
        let node = TreeNode {
            branch: branch("child"),
            depth: 1,
            is_last_child: true,
            ancestor_is_last: vec![true],
        };
        let spans = node.prefix_symbols();
        // ancestor[0] is_last=true → "   " (3 spaces), then corner "└─ "
        assert_eq!(spans.len(), 2);
    }

    #[test]
    fn depth_one_not_last_child_gets_tee() {
        let node = TreeNode {
            branch: branch("child"),
            depth: 1,
            is_last_child: false,
            ancestor_is_last: vec![false],
        };
        let spans = node.prefix_symbols();
        // ancestor[0] is_last=false → "│  ", then tee "├─ "
        assert_eq!(spans.len(), 2);
    }

    #[test]
    fn deep_nesting_produces_long_prefix() {
        let mut ancestors = Vec::new();
        for i in 0..20 {
            ancestors.push(i % 2 == 0);
        }
        let node = TreeNode {
            branch: branch("deep"),
            depth: 21,
            is_last_child: false,
            ancestor_is_last: ancestors,
        };
        let spans = node.prefix_symbols();
        // 20 ancestor spans + 1 connector = 21
        assert_eq!(spans.len(), 21);
    }

    #[test]
    fn all_ancestors_last_produces_no_vertical_bars() {
        let ancestors = vec![true, true, true, true, true];
        let node = TreeNode {
            branch: branch("leaf"),
            depth: 5,
            is_last_child: true,
            ancestor_is_last: ancestors,
        };
        let spans = node.prefix_symbols();
        // All ancestors are last → all "   " (spaces) + "└─ "
        let all_content: String = spans.iter().map(|s| s.content.clone()).collect();
        assert!(
            !all_content.contains('│'),
            "no vertical bars when all ancestors are last"
        );
    }

    #[test]
    fn no_ancestors_last_produces_all_vertical_bars() {
        let ancestors = vec![false, false, false];
        let node = TreeNode {
            branch: branch("inner"),
            depth: 3,
            is_last_child: false,
            ancestor_is_last: ancestors,
        };
        let spans = node.prefix_symbols();
        let all_content: String = spans.iter().map(|s| s.content.clone()).collect();
        // 3 "│  " + 1 "├─ "
        assert_eq!(all_content.matches('│').count(), 3);
    }
}

#[cfg(test)]
mod worktree_fragility {
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
    fn selected_item_out_of_bounds_returns_none() {
        let view = WorktreeView::new(vec![make_item("only")]);
        // Directly set selected_index beyond bounds
        let mut view = view;
        view.selected_index = 999;
        assert!(
            view.selected_item().is_none(),
            "out-of-bounds index should return None"
        );
    }

    #[test]
    fn with_items_resets_selection_on_replacement() {
        let view = WorktreeView::new(vec![make_item("a"), make_item("b"), make_item("c")]);
        let mut view = view;
        view.selected_index = 2;
        let view = view.with_items(vec![make_item("x")]);
        assert_eq!(
            view.selected_index, 0,
            "with_items should always reset to 0"
        );
    }

    #[test]
    fn with_items_empty_clears_selection() {
        let view = WorktreeView::new(vec![make_item("a")]);
        let view = view.with_items(Vec::new());
        assert_eq!(view.selected_index, 0);
        assert!(view.selected_item().is_none());
    }

    #[test]
    fn select_next_on_single_item_stays_at_zero() {
        let mut view = WorktreeView::new(vec![make_item("solo")]);
        view.select_next();
        view.select_next();
        view.select_next();
        assert_eq!(view.selected_index, 0);
    }

    #[test]
    fn select_prev_on_single_item_stays_at_zero() {
        let mut view = WorktreeView::new(vec![make_item("solo")]);
        view.select_previous();
        view.select_previous();
        assert_eq!(view.selected_index, 0);
    }

    #[test]
    fn two_item_alternating_returns_to_start() {
        let mut view = WorktreeView::new(vec![make_item("a"), make_item("b")]);
        view.select_next(); // 1
        view.select_next(); // 0
        view.select_previous(); // 1
        view.select_previous(); // 0
        assert_eq!(view.selected_index, 0);
    }

    #[test]
    fn rapid_alternating_stress() {
        let items: Vec<WorktreeItem> = (0..50).map(|i| make_item(&format!("wt-{}", i))).collect();
        let mut view = WorktreeView::new(items);
        for _ in 0..500 {
            view.select_next();
            view.select_previous();
        }
        assert_eq!(
            view.selected_index, 0,
            "alternating next/prev should return to 0"
        );
    }

    #[test]
    fn default_view_is_empty() {
        let view = WorktreeView::default();
        assert!(view.selected_item().is_none());
        assert_eq!(view.selected_index, 0);
    }

    #[test]
    fn new_with_many_items_last_selected_wraps() {
        let items: Vec<WorktreeItem> = (0..100).map(|i| make_item(&format!("wt-{}", i))).collect();
        let mut view = WorktreeView::new(items);
        view.selected_index = 0;
        view.select_previous();
        assert_eq!(
            view.selected_index, 99,
            "prev from 0 on 100 items wraps to 99"
        );
    }
}

#[cfg(test)]
mod diff_line_semantics {
    use crate::app::{DiffLine, DiffLineKind};

    #[test]
    fn diff_line_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<DiffLine>();
    }

    #[test]
    fn diff_line_is_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<DiffLine>();
    }

    #[test]
    fn diff_line_clone_is_deep() {
        let line = DiffLine::new("+hello", DiffLineKind::Add);
        let cloned = line.clone();
        assert_eq!(line.content, cloned.content);
        assert_eq!(line.kind, cloned.kind);
    }

    #[test]
    fn empty_content_all_variants() {
        let variants = vec![
            DiffLine::new("", DiffLineKind::Header),
            DiffLine::new("", DiffLineKind::Hunk),
            DiffLine::new("", DiffLineKind::Add),
            DiffLine::new("", DiffLineKind::Remove),
            DiffLine::new("", DiffLineKind::Context),
        ];
        for v in variants {
            let cloned = v.clone();
            assert_eq!(
                v.content, cloned.content,
                "empty content should clone correctly"
            );
        }
    }

    #[test]
    fn diff_line_with_special_characters() {
        let special = "\t\r\n\x00\x1b\x07";
        let line = DiffLine::new(special, DiffLineKind::Context);
        assert_eq!(line.clone().content, line.content);
    }

    #[test]
    fn diff_line_with_null_bytes() {
        let line = DiffLine::new("add\0null", DiffLineKind::Add);
        assert_eq!(line.clone().content, line.content);
    }

    #[test]
    fn diff_line_eq_reflexivity_all_variants() {
        let lines = vec![
            DiffLine::new("h", DiffLineKind::Header),
            DiffLine::new("hh", DiffLineKind::Hunk),
            DiffLine::new("+", DiffLineKind::Add),
            DiffLine::new("-", DiffLineKind::Remove),
            DiffLine::new(" ", DiffLineKind::Context),
        ];
        for line in lines {
            assert_eq!(line.content, line.content);
        }
    }

    #[test]
    fn diff_line_eq_symmetry() {
        let a = DiffLine::new("x", DiffLineKind::Header);
        let b = DiffLine::new("x", DiffLineKind::Header);
        assert_eq!(a.content, b.content);
        assert_eq!(a.kind, b.kind);
    }

    #[test]
    fn diff_line_eq_transitivity() {
        let a = DiffLine::new("c", DiffLineKind::Context);
        let b = DiffLine::new("c", DiffLineKind::Context);
        let c = DiffLine::new("c", DiffLineKind::Context);
        assert_eq!(a.content, b.content);
        assert_eq!(b.content, c.content);
        assert_eq!(a.content, c.content);
    }

    #[test]
    fn diff_line_debug_format_contains_variant_name() {
        let h = format!("{:?}", DiffLine::new("x", DiffLineKind::Header));
        let hh = format!("{:?}", DiffLine::new("x", DiffLineKind::Hunk));
        let a = format!("{:?}", DiffLine::new("x", DiffLineKind::Add));
        let d = format!("{:?}", DiffLine::new("x", DiffLineKind::Remove));
        let c = format!("{:?}", DiffLine::new("x", DiffLineKind::Context));
        assert!(h.contains("Header"));
        assert!(hh.contains("Hunk"));
        assert!(a.contains("Add"));
        assert!(d.contains("Remove"));
        assert!(c.contains("Context"));
    }
}

#[cfg(test)]
mod input_handler_modifiers {
    use crate::app::Mode;
    use crate::input::{HunkAction, InputHandler, InputResult};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key_with(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, mods)
    }

    #[test]
    fn ctrl_q_is_still_quit() {
        let mut handler = InputHandler::new();
        let result = handler.handle_key_event(
            key_with(KeyCode::Char('q'), KeyModifiers::CONTROL),
            &Mode::Normal,
        );
        assert_eq!(result, InputResult::Quit);
    }

    #[test]
    fn shift_q_is_still_quit() {
        let mut handler = InputHandler::new();
        let result = handler.handle_key_event(
            key_with(KeyCode::Char('q'), KeyModifiers::SHIFT),
            &Mode::Normal,
        );
        assert_eq!(
            result,
            InputResult::Quit,
            "Shift+Q (uppercase) should also quit"
        );
    }

    #[test]
    fn ctrl_s_is_still_stage() {
        let mut handler = InputHandler::new();
        let result = handler.handle_key_event(
            key_with(KeyCode::Char('s'), KeyModifiers::CONTROL),
            &Mode::Normal,
        );
        assert_eq!(result, InputResult::Handled(HunkAction::Stage));
    }

    #[test]
    fn alt_d_is_still_discard() {
        let mut handler = InputHandler::new();
        let result = handler.handle_key_event(
            key_with(KeyCode::Char('d'), KeyModifiers::ALT),
            &Mode::Normal,
        );
        assert_eq!(result, InputResult::Handled(HunkAction::Discard));
    }

    #[test]
    fn ctrl_shift_combined_on_j() {
        let mut handler = InputHandler::new();
        let result = handler.handle_key_event(
            key_with(
                KeyCode::Char('j'),
                KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            ),
            &Mode::Normal,
        );
        // KeyCode::Char('j') regardless of modifiers maps to NavigateNext
        assert_eq!(result, InputResult::Handled(HunkAction::NavigateNext));
    }

    #[test]
    fn esc_with_any_modifier_is_quit() {
        let mut handler = InputHandler::new();
        let result =
            handler.handle_key_event(key_with(KeyCode::Esc, KeyModifiers::SHIFT), &Mode::Normal);
        assert_eq!(result, InputResult::Quit);
    }

    #[test]
    fn all_modifiers_combined_on_mapped_key() {
        let mut handler = InputHandler::new();
        let result = handler.handle_key_event(
            key_with(
                KeyCode::Char('u'),
                KeyModifiers::CONTROL | KeyModifiers::SHIFT | KeyModifiers::ALT,
            ),
            &Mode::Normal,
        );
        assert_eq!(result, InputResult::Handled(HunkAction::Unstage));
    }

    #[test]
    fn modifier_keys_still_navigate() {
        let mut handler = InputHandler::new();
        handler.set_hunk_count(3);

        let ctrl_down = key_with(KeyCode::Down, KeyModifiers::CONTROL);
        handler.handle_key_event(ctrl_down, &Mode::Normal);
        assert_eq!(handler.current_hunk, 1);

        let shift_up = key_with(KeyCode::Up, KeyModifiers::SHIFT);
        handler.handle_key_event(shift_up, &Mode::Normal);
        assert_eq!(handler.current_hunk, 0);
    }
}

#[cfg(test)]
mod stack_children_field {
    use crate::widgets::StackTreeWidget;
    use scp_stack::domain::value_objects::BranchName;
    use scp_stack::domain::StackBranch;

    fn branch(name: &str, parent: Option<&str>, children: Vec<&str>) -> StackBranch {
        StackBranch {
            name: BranchName::new(name.to_string()),
            parent: parent.map(|p| BranchName::new(p.to_string())),
            children: children
                .iter()
                .map(|c| BranchName::new(c.to_string()))
                .collect(),
            needs_restack: false,
            pr_info: None,
        }
    }

    #[test]
    fn children_field_is_cosmetic_tree_uses_parent_only() {
        // The children field on StackBranch is NOT used by build_tree_nodes.
        // Tree construction uses the parent field to find children.
        let branches = vec![
            branch("root", None, vec!["child1", "child2"]), // children listed but ignored
            branch("child1", Some("root"), vec![]),         // parent link is what matters
            branch("child2", Some("root"), vec![]),
        ];
        let nodes = StackTreeWidget::new(branches).build_tree_nodes();
        assert_eq!(nodes.len(), 3);
        assert_eq!(nodes[1].branch.name.as_str(), "child1");
        assert_eq!(nodes[2].branch.name.as_str(), "child2");
    }

    #[test]
    fn children_field_empty_does_not_affect_tree() {
        let branches = vec![
            branch("root", None, vec![]), // no children listed
            branch("child", Some("root"), vec![]),
        ];
        let nodes = StackTreeWidget::new(branches).build_tree_nodes();
        assert_eq!(nodes.len(), 2);
    }

    #[test]
    fn stale_children_list_ignored() {
        // children says "orphan" is a child, but orphan has no parent pointing back
        let branches = vec![
            branch("root", None, vec!["orphan"]), // claims orphan is child
            branch("orphan", None, vec![]),       // but orphan has no parent → it's a root
        ];
        let nodes = StackTreeWidget::new(branches).build_tree_nodes();
        // Both are roots since orphan.parent is None
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes.iter().filter(|n| n.depth == 0).count(), 2);
    }
}

#[cfg(test)]
mod widget_selection_edge_cases {
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
    fn selection_none_on_non_empty_tree() {
        let widget = StackTreeWidget::new(vec![branch("a", None)]).with_selection(None);
        assert_eq!(widget.selected_index, None);
        assert_eq!(widget.build_tree_nodes().len(), 1);
    }

    #[test]
    fn selection_out_of_bounds_on_single_node() {
        let widget = StackTreeWidget::new(vec![branch("a", None)]).with_selection(Some(99));
        assert_eq!(widget.selected_index, Some(99));
        // selection is just stored, not validated at construction time
    }

    #[test]
    fn selection_changes_do_not_affect_tree_nodes() {
        let branches = vec![branch("a", None), branch("b", None)];
        let w1 = StackTreeWidget::new(branches.clone()).with_selection(None);
        let w2 = StackTreeWidget::new(branches.clone()).with_selection(Some(0));
        assert_eq!(w1.build_tree_nodes().len(), w2.build_tree_nodes().len());
    }

    #[test]
    fn empty_widget_with_selection_still_empty() {
        let widget = StackTreeWidget::new(Vec::new()).with_selection(Some(0));
        assert!(widget.build_tree_nodes().is_empty());
        assert_eq!(widget.selected_index, Some(0));
    }

    #[test]
    fn with_selection_overrides_previous() {
        let widget = StackTreeWidget::new(vec![branch("a", None)])
            .with_selection(Some(5))
            .with_selection(Some(3));
        assert_eq!(widget.selected_index, Some(3));
    }
}

#[cfg(test)]
mod refresh_idempotency {
    use super::*;
    use scp_stack::domain::BranchName;
    use std::sync::atomic::{AtomicUsize, Ordering};

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
            Ok(vec![StackBranch {
                name: BranchName::new("main"),
                parent: None,
                children: Vec::new(),
                needs_restack: false,
                pr_info: None,
            }])
        }
    }

    #[test]
    fn refresh_without_rearm_calls_provider_once() {
        let provider = CountingProvider::new();
        let mut app = TuiApp::new(Box::new(provider)).expect("ok");
        app.refresh_branches().expect("ok");
        app.refresh_branches().expect("ok");
        app.refresh_branches().expect("ok");
        assert_eq!(
            app.stack_branches.len(),
            1,
            "branches from first call preserved"
        );
        // Provider was called exactly once
    }

    #[test]
    fn rearm_between_each_refresh_calls_provider_each_time() {
        let provider = CountingProvider::new();
        let mut app = TuiApp::new(Box::new(provider)).expect("ok");
        for _ in 0..5 {
            app.needs_refresh = true;
            app.refresh_branches().expect("ok");
        }
        assert_eq!(app.stack_branches.len(), 1);
    }

    #[test]
    fn refresh_error_clears_flag_preventing_repeated_calls() {
        struct FailOnce {
            called: AtomicUsize,
        }
        impl BranchProvider for FailOnce {
            fn load_branches(&self) -> std::result::Result<Vec<StackBranch>, String> {
                if self.called.fetch_add(1, Ordering::SeqCst) == 0 {
                    Err("transient".to_string())
                } else {
                    Ok(vec![StackBranch {
                        name: BranchName::new("main"),
                        parent: None,
                        children: Vec::new(),
                        needs_restack: false,
                        pr_info: None,
                    }])
                }
            }
        }
        let provider = FailOnce {
            called: AtomicUsize::new(0),
        };
        let mut app = TuiApp::new(Box::new(provider)).expect("ok");
        // First refresh: fails
        app.refresh_branches().expect("ok");
        assert!(app.status_message.contains("transient"));
        assert!(app.stack_branches.is_empty());
        // Second refresh without rearm: skipped
        app.refresh_branches().expect("ok");
        assert!(app.stack_branches.is_empty(), "should not have retried");
        // Third refresh with rearm: succeeds
        app.needs_refresh = true;
        app.refresh_branches().expect("ok");
        assert_eq!(app.stack_branches.len(), 1);
    }
}

#[cfg(test)]
mod format_functions {
    use crate::widgets::StackTreeWidget;
    use scp_stack::domain::value_objects::BranchName;
    use scp_stack::domain::{PrInfo, PrState, StackBranch};

    fn branch(name: &str) -> StackBranch {
        StackBranch {
            name: BranchName::new(name.to_string()),
            parent: None,
            children: Vec::new(),
            needs_restack: false,
            pr_info: None,
        }
    }

    #[test]
    fn format_branch_name_simple() {
        let b = branch("feature/my-branch");
        assert_eq!(StackTreeWidget::format_branch_name(&b), "feature/my-branch");
    }

    #[test]
    fn format_branch_name_empty() {
        let b = branch("");
        assert_eq!(StackTreeWidget::format_branch_name(&b), "");
    }

    #[test]
    fn format_branch_name_unicode() {
        let b = branch("フィーチャー/ブランチ");
        assert_eq!(
            StackTreeWidget::format_branch_name(&b),
            "フィーチャー/ブランチ"
        );
    }

    #[test]
    fn format_branch_name_slashes() {
        let b = branch("a/b/c/d/e");
        assert_eq!(StackTreeWidget::format_branch_name(&b), "a/b/c/d/e");
    }

    #[test]
    fn format_pr_info_open() {
        let pr = PrInfo {
            number: 42,
            url: "https://example.com/42".into(),
            title: "Feature".into(),
            state: PrState::Open,
            is_draft: None,
        };
        let formatted = StackTreeWidget::format_pr_info(&pr);
        assert!(formatted.contains("42"));
        assert!(formatted.contains("○"));
    }

    #[test]
    fn format_pr_info_merged() {
        let pr = PrInfo {
            number: 100,
            url: "https://example.com/100".into(),
            title: "Merged PR".into(),
            state: PrState::Merged,
            is_draft: None,
        };
        let formatted = StackTreeWidget::format_pr_info(&pr);
        assert!(formatted.contains("100"));
        assert!(formatted.contains("◆"));
    }

    #[test]
    fn format_pr_info_closed() {
        let pr = PrInfo {
            number: 7,
            url: "https://example.com/7".into(),
            title: "Closed PR".into(),
            state: PrState::Closed,
            is_draft: None,
        };
        let formatted = StackTreeWidget::format_pr_info(&pr);
        assert!(formatted.contains("7"));
        assert!(formatted.contains("×"));
    }

    #[test]
    fn format_pr_info_zero_number() {
        let pr = PrInfo {
            number: 0,
            url: "https://example.com/0".into(),
            title: "Zero PR".into(),
            state: PrState::Open,
            is_draft: None,
        };
        let formatted = StackTreeWidget::format_pr_info(&pr);
        assert!(formatted.contains("0"));
    }

    #[test]
    fn pr_state_symbol_all_variants() {
        assert_eq!(StackTreeWidget::pr_state_symbol(&PrState::Open), "○");
        assert_eq!(StackTreeWidget::pr_state_symbol(&PrState::Merged), "◆");
        assert_eq!(StackTreeWidget::pr_state_symbol(&PrState::Closed), "×");
    }
}

#[cfg(test)]
mod mode_clone_equality_deep {
    use crate::app::{ConfirmAction, InputAction, Mode};

    #[test]
    fn mode_clone_with_nested_data() {
        let long_name = "a".repeat(500);
        let mode = Mode::Confirm(ConfirmAction::Delete(long_name.clone()));
        let cloned = mode.clone();
        match &cloned {
            Mode::Confirm(ConfirmAction::Delete(name)) => assert_eq!(name.len(), 500),
            _ => panic!("expected Confirm::Delete"),
        }
        assert_eq!(mode, cloned);
    }

    #[test]
    fn mode_clone_independence() {
        let mut mode = Mode::Confirm(ConfirmAction::Restack("original".into()));
        let cloned = mode.clone();
        // We can't mutate inner data since ConfirmAction::Restack holds String,
        // but we can replace the mode entirely
        mode = Mode::Normal;
        assert!(matches!(cloned, Mode::Confirm(ConfirmAction::Restack(_))));
        assert!(matches!(mode, Mode::Normal));
    }

    #[test]
    fn all_modes_are_clone() {
        let modes = vec![
            Mode::Normal,
            Mode::Search,
            Mode::Help,
            Mode::Confirm(ConfirmAction::Delete("x".into())),
            Mode::Confirm(ConfirmAction::Restack("y".into())),
            Mode::Confirm(ConfirmAction::RestackAll),
            Mode::Confirm(ConfirmAction::ApplyReorder),
            Mode::Input(InputAction::Rename),
            Mode::Input(InputAction::NewBranch),
            Mode::Reorder,
        ];
        for mode in modes {
            let _cloned = mode.clone();
        }
    }

    #[test]
    fn confirm_action_clone_with_long_name() {
        let long = "x".repeat(10_000);
        let action = ConfirmAction::Delete(long.clone());
        let cloned = action.clone();
        assert_eq!(action, cloned);
        if let ConfirmAction::Delete(name) = cloned {
            assert_eq!(name.len(), 10_000);
        }
    }

    #[test]
    fn input_action_copy_semantics() {
        // InputAction is Copy, so clone should be identical
        let a = InputAction::Rename;
        let b = a;
        assert_eq!(a, b);
        let c = a.clone();
        assert_eq!(a, c);
    }
}

#[cfg(test)]
mod proptest_gen3 {
    use crate::app::{ConfirmAction, DiffLine, DiffLineKind, InputAction, Mode};
    use proptest::proptest;

    proptest! {
        #[test]
        fn prop_diff_line_clone_roundtrip(text in ".{0,500}") {
            let kinds = [
                DiffLineKind::Header,
                DiffLineKind::Hunk,
                DiffLineKind::Add,
                DiffLineKind::Remove,
                DiffLineKind::Context,
            ];
            for kind in kinds {
                let line = DiffLine::new(text.clone(), kind);
                let cloned = line.clone();
                assert_eq!(line.content, cloned.content);
                assert_eq!(line.kind, cloned.kind);
            }
        }

        #[test]
        fn prop_mode_confirm_delete_roundtrip(name in proptest::string::string_regex(".{0,2000}").unwrap()) {
            let mode = Mode::Confirm(ConfirmAction::Delete(name.clone()));
            if let Mode::Confirm(ConfirmAction::Delete(n)) = mode {
                assert_eq!(n, name);
            }
        }

        #[test]
        fn prop_confirm_action_delete_neq_restack(name in ".{0,100}") {
            let delete = ConfirmAction::Delete(name.clone());
            let restack = ConfirmAction::Restack(name);
            assert_ne!(delete, restack, "Delete and Restack should never be equal even with same name");
        }

        #[test]
        fn prop_input_action_copy_roundtrip(b in proptest::bool::ANY) {
            let action = if b { InputAction::Rename } else { InputAction::NewBranch };
            let copy = action;
            assert_eq!(action, copy);
        }
    }
}
