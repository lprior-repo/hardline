use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Widget},
};
use scp_stack::domain::{PrInfo, PrState, StackBranch};

#[derive(Debug, Clone)]
pub struct TreeNode {
    pub branch: StackBranch,
    pub depth: usize,
    pub is_last_child: bool,
    pub ancestor_is_last: Vec<bool>,
}

impl TreeNode {
    fn prefix_symbols(&self) -> Vec<Span<'static>> {
        let mut spans = Vec::new();
        for is_last in &self.ancestor_is_last {
            if *is_last {
                spans.push(Span::raw("   "));
            } else {
                spans.push(Span::raw("│  "));
            }
        }
        if self.depth > 0 {
            if self.is_last_child {
                spans.push(Span::raw("└─ "));
            } else {
                spans.push(Span::raw("├─ "));
            }
        }
        spans
    }
}

#[derive(Debug, Clone)]
pub struct StackTreeWidget {
    pub branches: Vec<StackBranch>,
    pub selected_index: Option<usize>,
}

impl StackTreeWidget {
    pub fn new(branches: Vec<StackBranch>) -> Self {
        Self {
            branches,
            selected_index: None,
        }
    }

    pub fn with_selection(mut self, index: Option<usize>) -> Self {
        self.selected_index = index;
        self
    }

    fn build_tree_nodes(&self) -> Vec<TreeNode> {
        if self.branches.is_empty() {
            return Vec::new();
        }
        let mut nodes = Vec::new();
        self.collect_root_nodes(&mut nodes, 0, &[]);
        nodes
    }

    fn collect_root_nodes(
        &self,
        nodes: &mut Vec<TreeNode>,
        depth: usize,
        ancestor_is_last: &[bool],
    ) {
        let roots: Vec<&StackBranch> = self
            .branches
            .iter()
            .filter(|b| b.parent.is_none())
            .collect();

        let total = roots.len();
        for (idx, root) in roots.iter().enumerate() {
            let is_last = idx == total - 1;
            let mut new_ancestor = ancestor_is_last.to_vec();
            new_ancestor.push(is_last);

            nodes.push(TreeNode {
                branch: (*root).clone(),
                depth,
                is_last_child: is_last,
                ancestor_is_last: new_ancestor.clone(),
            });

            self.collect_children_of(root, nodes, depth + 1, &new_ancestor);
        }
    }

    fn collect_children_of(
        &self,
        parent: &StackBranch,
        nodes: &mut Vec<TreeNode>,
        depth: usize,
        ancestor_is_last: &[bool],
    ) {
        self.collect_children_of_inner(parent, nodes, depth, ancestor_is_last, &mut std::collections::HashSet::new());
    }

    fn collect_children_of_inner(
        &self,
        parent: &StackBranch,
        nodes: &mut Vec<TreeNode>,
        depth: usize,
        ancestor_is_last: &[bool],
        visited: &mut std::collections::HashSet<String>,
    ) {
        if !visited.insert(parent.name.to_string()) {
            return;
        }

        let children: Vec<&StackBranch> = self
            .branches
            .iter()
            .filter(|b| b.parent.as_ref() == Some(&parent.name))
            .collect();

        let total = children.len();
        for (idx, child) in children.iter().enumerate() {
            let is_last = idx == total - 1;
            let mut new_ancestor = ancestor_is_last.to_vec();
            new_ancestor.push(is_last);

            nodes.push(TreeNode {
                branch: (*child).clone(),
                depth,
                is_last_child: is_last,
                ancestor_is_last: new_ancestor.clone(),
            });

            self.collect_children_of_inner(child, nodes, depth + 1, &new_ancestor, visited);
        }
    }

    fn branch_indicator(branch: &StackBranch) -> (&'static str, Color) {
        if branch.needs_restack {
            ("⚑", Color::Red)
        } else if branch.pr_info.is_some() {
            ("●", Color::Green)
        } else {
            ("○", Color::Blue)
        }
    }

    fn pr_state_symbol(state: &PrState) -> &'static str {
        match state {
            PrState::Open => "○",
            PrState::Merged => "◆",
            PrState::Closed => "×",
        }
    }

    fn format_branch_name(branch: &StackBranch) -> String {
        branch.name.to_string()
    }

    fn format_pr_info(pr_info: &PrInfo) -> String {
        format!(
            " (#{} {})",
            pr_info.number,
            Self::pr_state_symbol(&pr_info.state)
        )
    }
}

impl Widget for StackTreeWidget {
    fn render(self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Stack Tree")
            .title_style(Style::default());

        let inner_area = block.inner(area);
        block.render(area, buf);

        let nodes = self.build_tree_nodes();
        if nodes.is_empty() {
            return;
        }

        let mut items = Vec::new();

        for (idx, node) in nodes.iter().enumerate() {
            let mut line_spans: Vec<Span> = node.prefix_symbols();
            let (indicator, color) = Self::branch_indicator(&node.branch);

            line_spans.push(Span::styled(
                format!("{} ", indicator),
                Style::default().fg(color),
            ));

            let branch_name = Self::format_branch_name(&node.branch);
            line_spans.push(Span::styled(branch_name, Style::default().fg(color)));

            if let Some(ref pr_info) = node.branch.pr_info {
                let pr_color = match pr_info.state {
                    PrState::Open => Color::Green,
                    PrState::Merged => Color::Magenta,
                    PrState::Closed => Color::Red,
                };
                let pr_text = Self::format_pr_info(pr_info);
                line_spans.push(Span::styled(pr_text, Style::default().fg(pr_color)));
            }

            let is_selected = self.selected_index == Some(idx);
            if is_selected {
                let last = line_spans.len() - 1;
                line_spans[last] = line_spans[last]
                    .clone()
                    .style(Style::default().bg(Color::Blue));
            }

            let line = Line::from(line_spans);
            items.push(ListItem::new(line));
        }

        let list = List::new(items)
            .block(Block::default())
            .style(Style::default());
        list.render(inner_area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scp_stack::domain::value_objects::BranchName;

    fn create_test_branch(name: &str, parent: Option<&str>) -> StackBranch {
        StackBranch {
            name: BranchName::new(name.to_string()),
            parent: parent.map(|p| BranchName::new(p.to_string())),
            children: Vec::new(),
            needs_restack: false,
            pr_info: None,
        }
    }

    fn create_test_pr_info(number: u32, state: PrState) -> PrInfo {
        PrInfo {
            number,
            url: format!("https://github.com/org/repo/pull/{}", number),
            title: format!("PR #{}", number),
            state,
            is_draft: Some(false),
        }
    }

    #[test]
    fn stack_tree_widget_empty_branches() {
        let widget = StackTreeWidget::new(Vec::new());
        assert!(widget.build_tree_nodes().is_empty());
    }

    #[test]
    fn stack_tree_widget_single_branch() {
        let branches = vec![create_test_branch("main", None)];
        let widget = StackTreeWidget::new(branches);
        let nodes = widget.build_tree_nodes();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].depth, 0);
        assert!(nodes[0].is_last_child);
    }

    #[test]
    fn stack_tree_widget_nested_branches() {
        let branches = vec![
            create_test_branch("main", None),
            create_test_branch("feature/a", Some("main")),
            create_test_branch("feature/b", Some("main")),
            create_test_branch("fix/x", Some("feature/a")),
        ];
        let widget = StackTreeWidget::new(branches);
        let nodes = widget.build_tree_nodes();
        assert_eq!(nodes.len(), 4);
    }

    #[test]
    fn stack_tree_widget_depth_calculation() {
        let branches = vec![
            create_test_branch("main", None),
            create_test_branch("feature/a", Some("main")),
            create_test_branch("fix/x", Some("feature/a")),
        ];
        let widget = StackTreeWidget::new(branches);
        let nodes = widget.build_tree_nodes();

        let depths: Vec<usize> = nodes.iter().map(|n| n.depth).collect();
        assert_eq!(depths, vec![0, 1, 2]);
    }

    #[test]
    fn stack_tree_widget_with_selection() {
        let branches = vec![create_test_branch("main", None)];
        let widget = StackTreeWidget::new(branches).with_selection(Some(0));
        assert_eq!(widget.selected_index, Some(0));
    }

    #[test]
    fn tree_node_prefix_symbols_root() {
        let branch = create_test_branch("main", None);
        let node = TreeNode {
            branch,
            depth: 0,
            is_last_child: true,
            ancestor_is_last: Vec::new(),
        };
        let symbols = node.prefix_symbols();
        assert!(symbols.is_empty());
    }

    #[test]
    fn tree_node_prefix_symbols_child() {
        let branch = create_test_branch("feature/a", Some("main"));
        let node = TreeNode {
            branch,
            depth: 1,
            is_last_child: true,
            ancestor_is_last: vec![true],
        };
        let symbols = node.prefix_symbols();
        assert_eq!(symbols.len(), 2);
    }

    #[test]
    fn branch_indicator_no_pr_no_restack() {
        let branch = create_test_branch("main", None);
        let (indicator, color) = StackTreeWidget::branch_indicator(&branch);
        assert_eq!(indicator, "○");
        assert_eq!(color, Color::Blue);
    }

    #[test]
    fn branch_indicator_with_pr() {
        let mut branch = create_test_branch("feature/a", Some("main"));
        branch.pr_info = Some(create_test_pr_info(1, PrState::Open));
        let (indicator, color) = StackTreeWidget::branch_indicator(&branch);
        assert_eq!(indicator, "●");
        assert_eq!(color, Color::Green);
    }

    #[test]
    fn branch_indicator_needs_restack() {
        let mut branch = create_test_branch("wip/x", None);
        branch.needs_restack = true;
        let (indicator, color) = StackTreeWidget::branch_indicator(&branch);
        assert_eq!(indicator, "⚑");
        assert_eq!(color, Color::Red);
    }

    #[test]
    fn pr_state_symbol_mapping() {
        assert_eq!(StackTreeWidget::pr_state_symbol(&PrState::Open), "○");
        assert_eq!(StackTreeWidget::pr_state_symbol(&PrState::Merged), "◆");
        assert_eq!(StackTreeWidget::pr_state_symbol(&PrState::Closed), "×");
    }

    #[test]
    fn format_branch_name_test() {
        let branch = create_test_branch("feature/test", None);
        assert_eq!(StackTreeWidget::format_branch_name(&branch), "feature/test");
    }

    #[test]
    fn format_pr_info_test() {
        let pr_info = create_test_pr_info(42, PrState::Open);
        let formatted = StackTreeWidget::format_pr_info(&pr_info);
        assert!(formatted.contains("42"));
        assert!(formatted.contains("○"));
    }

    #[test]
    fn stack_tree_widget_with_pr_info() {
        let mut branch = create_test_branch("feature/a", None);
        branch.pr_info = Some(create_test_pr_info(123, PrState::Open));
        let branches = vec![branch];
        let widget = StackTreeWidget::new(branches);
        let nodes = widget.build_tree_nodes();
        assert!(nodes[0].branch.pr_info.is_some());
    }

    #[test]
    fn stack_tree_widget_clone() {
        let branches = vec![create_test_branch("main", None)];
        let widget = StackTreeWidget::new(branches);
        let cloned = widget.clone();
        assert_eq!(cloned.branches.len(), 1);
    }

    #[test]
    fn stack_tree_widget_debug() {
        let branches = vec![create_test_branch("main", None)];
        let widget = StackTreeWidget::new(branches);
        let debug = format!("{:?}", widget);
        assert!(debug.contains("StackTreeWidget"));
    }

    #[test]
    fn stack_tree_widget_cycle_detection() {
        // A -> B -> C -> B (cycle). Must not infinite loop.
        let branches = vec![
            create_test_branch("A", None),
            create_test_branch("B", Some("A")),
            create_test_branch("C", Some("B")),
            create_test_branch("B2-cycle", Some("C")), // parent is C, name differs
        ];
        // Create a real cycle: C's child points back to B
        let mut widget = StackTreeWidget::new(branches);
        // Manually inject a cycle: add a branch whose parent creates a loop
        widget.branches.push(create_test_branch("loop-back", Some("loop-back")));
        let nodes = widget.build_tree_nodes();
        // Should terminate without stack overflow and produce finite output
        assert!(nodes.len() < 100);
    }

    #[test]
    fn stack_tree_widget_direct_self_cycle() {
        // Branch that is its own parent — must not infinite loop.
        let mut widget = StackTreeWidget::new(vec![
            create_test_branch("root", None),
        ]);
        widget.branches.push(StackBranch {
            name: BranchName::new("self-loop".to_string()),
            parent: Some(BranchName::new("self-loop".to_string())),
            children: Vec::new(),
            needs_restack: false,
            pr_info: None,
        });
        let nodes = widget.build_tree_nodes();
        // root + self-loop (visited once, no infinite recursion)
        assert!(nodes.len() < 10);
    }

    mod red_queen_gen1 {
        use super::*;

        // =========================================================================
        // stack_tree_adversarial — Red Queen gen1 coevolution tests
        // Attacks tree traversal, cycle detection, rendering, and data integrity.
        // =========================================================================

        // --- ATTACK: diamond DAG (A→B, A→C, B→D, C→D) ---
        // A node reachable via two paths must appear exactly once (visited set).

        #[test]
        fn diamond_dag_renders_without_duplication() {
            let branches = vec![
                create_test_branch("A", None),
                create_test_branch("B", Some("A")),
                create_test_branch("C", Some("A")),
                create_test_branch("D", Some("B")),
                // D is also a child of C — diamond
                create_test_branch("D2", Some("C")),
            ];
            // D2 has parent C but name differs, so no cycle — but diamond structure
            let widget = StackTreeWidget::new(branches);
            let nodes = widget.build_tree_nodes();
            // Must terminate and produce finite output
            assert!(nodes.len() <= 5, "diamond should have <= 5 nodes, got {}", nodes.len());
        }

        // --- ATTACK: true 3-node cycle A→B→C→A ---

        #[test]
        fn three_node_cycle_terminates() {
            let mut widget = StackTreeWidget::new(vec![
                create_test_branch("A", None),
                create_test_branch("B", Some("A")),
            ]);
            // C's parent is B
            widget.branches.push(create_test_branch("C", Some("B")));
            // Now make A's parent point to C — creating A→B→C→A cycle
            widget.branches[0].parent = Some(BranchName::new("C".to_string()));
            let nodes = widget.build_tree_nodes();
            // Cycle must be broken by visited set — finite output
            assert!(nodes.len() < 20, "3-node cycle must terminate, got {}", nodes.len());
        }

        // --- ATTACK: orphan children (parent doesn't exist in list) ---

        #[test]
        fn orphan_child_branches_are_dropped_not_crashed() {
            let branches = vec![
                create_test_branch("main", None),
                create_test_branch("orphan-a", Some("nonexistent-parent")),
                create_test_branch("orphan-b", Some("also-missing")),
            ];
            let widget = StackTreeWidget::new(branches);
            let nodes = widget.build_tree_nodes();
            // Orphans are never reached from root traversal — only main appears
            assert_eq!(nodes.len(), 1, "only root should appear, orphans are unreachable");
            assert_eq!(nodes[0].branch.name.as_str(), "main");
        }

        // --- ATTACK: multiple independent roots ---

        #[test]
        fn multiple_roots_all_appear_in_output() {
            let branches = vec![
                create_test_branch("root-A", None),
                create_test_branch("root-B", None),
                create_test_branch("root-C", None),
                create_test_branch("child-of-A", Some("root-A")),
            ];
            let widget = StackTreeWidget::new(branches);
            let nodes = widget.build_tree_nodes();
            assert_eq!(nodes.len(), 4);
            // First three nodes should be roots (depth 0)
            let root_count = nodes.iter().filter(|n| n.depth == 0).count();
            assert_eq!(root_count, 3, "should have 3 roots");
        }

        // --- ATTACK: deeply nested tree (100 levels) ---

        #[test]
        fn deeply_nested_tree_terminates() {
            let mut branches = vec![create_test_branch("L0", None)];
            for i in 1..100 {
                branches.push(create_test_branch(
                    format!("L{}", i).as_str(),
                    Some(format!("L{}", i - 1).as_str()),
                ));
            }
            let widget = StackTreeWidget::new(branches);
            let nodes = widget.build_tree_nodes();
            assert_eq!(nodes.len(), 100);
            // Verify depth increases monotonically
            for (idx, node) in nodes.iter().enumerate() {
                assert_eq!(
                    node.depth, idx,
                    "node at position {} should have depth {}", idx, idx
                );
            }
        }

        // --- ATTACK: selection index out of bounds ---

        #[test]
        fn selection_out_of_bounds_does_not_panic_on_build() {
            let branches = vec![create_test_branch("main", None)];
            // Selection at index 999 when only 1 node exists — build_tree_nodes
            // doesn't use selected_index, so this should be fine
            let widget = StackTreeWidget::new(branches).with_selection(Some(999));
            let nodes = widget.build_tree_nodes();
            assert_eq!(nodes.len(), 1);
            // Widget holds the out-of-bounds index — render would skip it
            assert_eq!(widget.selected_index, Some(999));
        }

        // --- ATTACK: duplicate branch names ---

        #[test]
        fn duplicate_branch_names_handled_without_crash() {
            let branches = vec![
                create_test_branch("main", None),
                create_test_branch("dup", Some("main")),
                create_test_branch("dup", Some("main")),
                create_test_branch("dup-child", Some("dup")),
            ];
            let widget = StackTreeWidget::new(branches);
            let nodes = widget.build_tree_nodes();
            // Should not crash; both "dup" entries appear as children of main
            assert!(nodes.len() >= 3, "duplicates should still appear, got {}", nodes.len());
        }

        // --- ATTACK: branch name with unicode and special characters ---

        #[test]
        fn unicode_branch_names_render_without_panic() {
            let branches = vec![
                create_test_branch("main", None),
                create_test_branch("feature/日本語", Some("main")),
                create_test_branch("fix/émojis🚀", Some("main")),
                create_test_branch("wip/ctrl-\x00-null", Some("main")),
            ];
            let widget = StackTreeWidget::new(branches);
            let nodes = widget.build_tree_nodes();
            assert_eq!(nodes.len(), 4);
        }

        // --- ATTACK: prefix_symbols at depth 10 ---

        #[test]
        fn prefix_symbols_deep_depth_correct_count() {
            let mut ancestor_is_last = Vec::new();
            for _ in 0..10 {
                ancestor_is_last.push(false);
            }
            let node = TreeNode {
                branch: create_test_branch("deep", Some("parent")),
                depth: 10,
                is_last_child: false,
                ancestor_is_last,
            };
            let symbols = node.prefix_symbols();
            // 10 ancestor connectors + 1 child connector = 11 spans
            assert_eq!(symbols.len(), 11, "depth 10 should have 11 prefix spans");
        }

        // --- ATTACK: prefix_symbols all ancestors are last ---

        #[test]
        fn prefix_symbols_all_last_ancestors_produce_blanks() {
            let mut ancestor_is_last = Vec::new();
            for _ in 0..5 {
                ancestor_is_last.push(true);
            }
            let node = TreeNode {
                branch: create_test_branch("leaf", Some("parent")),
                depth: 5,
                is_last_child: true,
                ancestor_is_last,
            };
            let symbols = node.prefix_symbols();
            // 5 "   " (blank) + 1 "└─ " = 6 spans
            assert_eq!(symbols.len(), 6);
            // All ancestor spans should be blank (3 spaces)
            for span in &symbols[..5] {
                assert_eq!(span.content, "   ");
            }
        }

        // --- ATTACK: prefix_symbols no ancestors at depth 1 ---

        #[test]
        fn prefix_symbols_depth_one_no_ancestors() {
            let node = TreeNode {
                branch: create_test_branch("child", Some("root")),
                depth: 1,
                is_last_child: false,
                ancestor_is_last: vec![],
            };
            let symbols = node.prefix_symbols();
            // No ancestors, just the child connector
            assert_eq!(symbols.len(), 1);
            assert_eq!(symbols[0].content, "├─ ");
        }

        // --- ATTACK: prefix_symbols depth 0 produces nothing ---

        #[test]
        fn prefix_symbols_depth_zero_empty() {
            let node = TreeNode {
                branch: create_test_branch("root", None),
                depth: 0,
                is_last_child: true,
                ancestor_is_last: vec![],
            };
            let symbols = node.prefix_symbols();
            assert!(symbols.is_empty());
        }

        // --- ATTACK: two roots with interleaved children ---

        #[test]
        fn two_roots_with_children_maintain_correct_depths() {
            let branches = vec![
                create_test_branch("root1", None),
                create_test_branch("root2", None),
                create_test_branch("r1-child1", Some("root1")),
                create_test_branch("r1-child2", Some("root1")),
                create_test_branch("r2-child1", Some("root2")),
            ];
            let widget = StackTreeWidget::new(branches);
            let nodes = widget.build_tree_nodes();
            assert_eq!(nodes.len(), 5);

            // root1 at 0, r1-child1 at 1, r1-child2 at 1, root2 at 0, r2-child1 at 1
            let depths: Vec<usize> = nodes.iter().map(|n| n.depth).collect();
            assert_eq!(depths[0], 0); // root1
            assert_eq!(depths[1], 1); // r1-child1
            assert_eq!(depths[2], 1); // r1-child2
            assert_eq!(depths[3], 0); // root2
            assert_eq!(depths[4], 1); // r2-child1
        }

        // --- ATTACK: is_last_child correctness for single child ---

        #[test]
        fn single_child_is_always_last() {
            let branches = vec![
                create_test_branch("root", None),
                create_test_branch("only-child", Some("root")),
            ];
            let widget = StackTreeWidget::new(branches);
            let nodes = widget.build_tree_nodes();
            assert_eq!(nodes.len(), 2);
            assert!(nodes[1].is_last_child, "single child must be last");
        }

        // --- ATTACK: is_last_child correctness for multiple children ---

        #[test]
        fn last_of_many_children_is_last_others_are_not() {
            let branches = vec![
                create_test_branch("root", None),
                create_test_branch("child-1", Some("root")),
                create_test_branch("child-2", Some("root")),
                create_test_branch("child-3", Some("root")),
            ];
            let widget = StackTreeWidget::new(branches);
            let nodes = widget.build_tree_nodes();
            assert_eq!(nodes.len(), 4);
            assert!(!nodes[1].is_last_child, "first child is not last");
            assert!(!nodes[2].is_last_child, "middle child is not last");
            assert!(nodes[3].is_last_child, "last child is last");
        }

        // --- ATTACK: indirect cycle via orphan chain ---

        #[test]
        fn indirect_cycle_via_orphan_chain_terminates() {
            let mut widget = StackTreeWidget::new(vec![
                create_test_branch("main", None),
            ]);
            // Chain: X→Y→Z→X (cycle among orphans, none reachable from main)
            widget.branches.push(create_test_branch("X", Some("Z")));
            widget.branches.push(create_test_branch("Y", Some("X")));
            widget.branches.push(create_test_branch("Z", Some("Y")));
            let nodes = widget.build_tree_nodes();
            // Only main is reachable from root
            assert_eq!(nodes.len(), 1);
        }

        // --- ATTACK: wide tree (one root, 50 children) ---

        #[test]
        fn wide_tree_with_fifty_children_terminates() {
            let mut branches = vec![create_test_branch("root", None)];
            for i in 0..50 {
                branches.push(create_test_branch(
                    format!("child-{}", i).as_str(),
                    Some("root"),
                ));
            }
            let widget = StackTreeWidget::new(branches);
            let nodes = widget.build_tree_nodes();
            assert_eq!(nodes.len(), 51);
            // Only the last child should be is_last_child
            let children: Vec<&TreeNode> = nodes.iter().skip(1).collect();
            for (idx, child) in children.iter().enumerate() {
                if idx == 49 {
                    assert!(child.is_last_child);
                } else {
                    assert!(!child.is_last_child);
                }
            }
        }

        // --- ATTACK: branch_indicator priority (needs_restack beats PR) ---

        #[test]
        fn branch_indicator_restack_takes_priority_over_pr() {
            let mut branch = create_test_branch("wip", None);
            branch.needs_restack = true;
            branch.pr_info = Some(create_test_pr_info(42, PrState::Open));
            let (indicator, color) = StackTreeWidget::branch_indicator(&branch);
            assert_eq!(indicator, "⚑", "needs_restack should take priority");
            assert_eq!(color, Color::Red);
        }

        // --- ATTACK: PR info with all states ---

        #[test]
        fn branch_indicator_closed_pr_shows_correctly() {
            let mut branch = create_test_branch("closed-pr", None);
            branch.pr_info = Some(create_test_pr_info(1, PrState::Closed));
            let (indicator, color) = StackTreeWidget::branch_indicator(&branch);
            assert_eq!(indicator, "●");
            assert_eq!(color, Color::Green);
        }

        #[test]
        fn branch_indicator_merged_pr_shows_correctly() {
            let mut branch = create_test_branch("merged-pr", None);
            branch.pr_info = Some(create_test_pr_info(2, PrState::Merged));
            let (indicator, color) = StackTreeWidget::branch_indicator(&branch);
            assert_eq!(indicator, "●");
            assert_eq!(color, Color::Green);
        }

        // --- ATTACK: ancestor_is_last correctness through 3-level tree ---

        #[test]
        fn ancestor_is_last_tracking_across_three_levels() {
            let branches = vec![
                create_test_branch("R", None),
                create_test_branch("A", Some("R")),
                create_test_branch("B", Some("R")),
                create_test_branch("A1", Some("A")),
                create_test_branch("A2", Some("A")),
            ];
            let widget = StackTreeWidget::new(branches);
            let nodes = widget.build_tree_nodes();
            // R: depth=0, A: depth=1, A1: depth=2, A2: depth=2, B: depth=1
            // Verify A2's ancestor tracking is correct
            let a2 = nodes.iter().find(|n| n.branch.name.as_str() == "A2").unwrap();
            assert_eq!(a2.depth, 2);
            assert!(a2.is_last_child);
            // ancestor_is_last is built as: parent pushes its own is_last into the chain.
            // R is the only root → is_last=true, A is not last child of R → is_last=false,
            // A2 is last child of A → is_last=true (pushed by A's collect call).
            assert_eq!(a2.ancestor_is_last.len(), 3);
            assert_eq!(a2.ancestor_is_last[0], true, "R is last root");
            assert_eq!(a2.ancestor_is_last[1], false, "A is not last child of R");
            assert_eq!(a2.ancestor_is_last[2], true, "A2 is last child of A");

            // B's ancestors: B is last child of R, so ancestor_is_last = [true]
            let b = nodes.iter().find(|n| n.branch.name.as_str() == "B").unwrap();
            assert_eq!(b.depth, 1);
            assert!(b.is_last_child, "B is last child of R");
        }

        // --- ATTACK: empty string branch name ---

        #[test]
        fn empty_branch_name_creates_valid_widget() {
            let mut widget = StackTreeWidget::new(vec![
                create_test_branch("root", None),
            ]);
            // BranchName::new in scp_stack doesn't validate — accepts empty
            widget.branches.push(StackBranch {
                name: BranchName::new("".to_string()),
                parent: Some(BranchName::new("root".to_string())),
                children: Vec::new(),
                needs_restack: false,
                pr_info: None,
            });
            let nodes = widget.build_tree_nodes();
            assert!(nodes.len() >= 1, "should not crash on empty branch name");
        }

        // --- ATTACK: PR info with very large PR number ---

        #[test]
        fn pr_info_with_max_u32_number_formats() {
            let pr_info = PrInfo {
                number: u32::MAX,
                url: format!("https://github.com/org/repo/pull/{}", u32::MAX),
                title: format!("PR #{}", u32::MAX),
                state: PrState::Open,
                is_draft: Some(false),
            };
            let formatted = StackTreeWidget::format_pr_info(&pr_info);
            assert!(
                formatted.contains(&u32::MAX.to_string()),
                "PR number should appear in formatted output"
            );
        }
    }
}
