use std::collections::HashSet;

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

    pub(crate) fn build_tree_nodes(&self) -> Vec<TreeNode> {
        if self.branches.is_empty() {
            return Vec::new();
        }
        let mut nodes = Vec::new();
        let mut visited = HashSet::new();
        self.collect_root_nodes(&mut nodes, 0, &[], &mut visited);
        nodes
    }

    fn collect_root_nodes(
        &self,
        nodes: &mut Vec<TreeNode>,
        depth: usize,
        ancestor_is_last: &[bool],
        visited: &mut HashSet<usize>,
    ) {
        let roots: Vec<(usize, &StackBranch)> = self
            .branches
            .iter()
            .enumerate()
            .filter(|(_, b)| b.parent.is_none())
            .collect();

        let total = roots.len();
        for (idx, (branch_idx, root)) in roots.iter().enumerate() {
            if !visited.insert(*branch_idx) {
                continue;
            }
            let is_last = idx == total - 1;
            let mut new_ancestor = ancestor_is_last.to_vec();
            new_ancestor.push(is_last);

            nodes.push(TreeNode {
                branch: (*root).clone(),
                depth,
                is_last_child: is_last,
                ancestor_is_last: new_ancestor.clone(),
            });

            self.collect_children_of(*branch_idx, nodes, depth + 1, &new_ancestor, visited);
        }
    }

    fn collect_children_of(
        &self,
        parent_idx: usize,
        nodes: &mut Vec<TreeNode>,
        depth: usize,
        ancestor_is_last: &[bool],
        visited: &mut HashSet<usize>,
    ) {
        let parent_name = &self.branches[parent_idx].name;
        let children: Vec<(usize, &StackBranch)> = self
            .branches
            .iter()
            .enumerate()
            .filter(|(idx, b)| *idx != parent_idx && b.parent.as_ref() == Some(parent_name))
            .collect();

        let total = children.len();
        for (idx, (branch_idx, child)) in children.iter().enumerate() {
            if !visited.insert(*branch_idx) {
                continue;
            }
            let is_last = idx == total - 1;
            let mut new_ancestor = ancestor_is_last.to_vec();
            new_ancestor.push(is_last);

            nodes.push(TreeNode {
                branch: (*child).clone(),
                depth,
                is_last_child: is_last,
                ancestor_is_last: new_ancestor.clone(),
            });

            self.collect_children_of(*branch_idx, nodes, depth + 1, &new_ancestor, visited);
        }
    }

    pub(crate) fn branch_indicator(branch: &StackBranch) -> (&'static str, Color) {
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
}
