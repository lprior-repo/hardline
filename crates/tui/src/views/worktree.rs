use crate::widgets::worktree::WorktreeItem;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

#[cfg(test)]
use worktree::WorktreeState;

#[derive(Debug)]
pub struct WorktreeView {
    items: Vec<WorktreeItem>,
    selected_index: usize,
}

impl WorktreeView {
    pub fn new(items: Vec<WorktreeItem>) -> Self {
        Self {
            items,
            selected_index: 0,
        }
    }

    pub fn with_items(mut self, items: Vec<WorktreeItem>) -> Self {
        self.items = items;
        self.selected_index = 0;
        self
    }

    pub fn selected_item(&self) -> Option<&WorktreeItem> {
        self.items.get(self.selected_index)
    }

    pub fn select_next(&mut self) {
        if !self.items.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.items.len();
        }
    }

    pub fn select_previous(&mut self) {
        if !self.items.is_empty() {
            self.selected_index = if self.selected_index == 0 {
                self.items.len() - 1
            } else {
                self.selected_index - 1
            };
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .split(area);

        self.render_list(frame, chunks[0]);
        self.render_status(frame, chunks[1]);
    }

    fn render_list(&self, frame: &mut Frame, area: Rect) {
        let list_items: Vec<ListItem> = self
            .items
            .iter()
            .enumerate()
            .map(|(idx, item)| {
                let content = if idx == self.selected_index {
                    Line::from(vec![
                        Span::styled("> ", Style::new().fg(Color::Yellow)),
                        Span::raw(format!(
                            "{} ({}) [{}]",
                            item.name,
                            item.branch_label(),
                            item.state_label()
                        )),
                    ])
                } else {
                    Line::from(vec![
                        Span::raw("  "),
                        Span::raw(format!(
                            "{} ({}) [{}]",
                            item.name,
                            item.branch_label(),
                            item.state_label()
                        )),
                    ])
                };
                ListItem::new(content)
            })
            .collect();

        let list = List::new(list_items)
            .block(Block::default().borders(Borders::ALL).title("Worktrees"))
            .style(Style::new().bg(Color::DarkGray));

        frame.render_widget(list, area);
    }

    fn render_status(&self, frame: &mut Frame, area: Rect) {
        let selected = self.selected_item();
        let status_text = match selected {
            Some(item) => format!("Selected: {} at {}", item.name, item.path),
            None => "No worktrees available".to_string(),
        };

        let paragraph = Paragraph::new(Line::from(status_text))
            .block(Block::default().borders(Borders::ALL).title("Status"))
            .style(Style::new().bg(Color::Blue).fg(Color::White));

        frame.render_widget(paragraph, area);
    }

    #[cfg(test)]
    fn test_item(name: &str, state: WorktreeState) -> WorktreeItem {
        WorktreeItem {
            id: format!("test-id-{}", name),
            name: name.to_string(),
            path: format!("/tmp/{}", name),
            branch: Some("main".to_string()),
            state,
            is_active: false,
        }
    }
}

impl Default for WorktreeView {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worktree_view_new_creates_empty_view() {
        let view = WorktreeView::new(Vec::new());
        assert!(view.items.is_empty());
        assert_eq!(view.selected_index, 0);
        assert!(view.selected_item().is_none());
    }

    #[test]
    fn worktree_view_with_items_sets_items_and_resets_selection() {
        let items = vec![
            WorktreeView::test_item("wt1", WorktreeState::Active),
            WorktreeView::test_item("wt2", WorktreeState::Suspended),
        ];
        let view = WorktreeView::new(Vec::new()).with_items(items.clone());

        assert_eq!(view.items, items);
        assert_eq!(view.selected_index, 0);
    }

    #[test]
    fn worktree_view_select_next_wraps_around() {
        let items = vec![
            WorktreeView::test_item("wt1", WorktreeState::Active),
            WorktreeView::test_item("wt2", WorktreeState::Suspended),
        ];
        let mut view = WorktreeView::new(items);

        assert_eq!(view.selected_index, 0);
        view.select_next();
        assert_eq!(view.selected_index, 1);
        view.select_next();
        assert_eq!(view.selected_index, 0);
    }

    #[test]
    fn worktree_view_select_previous_wraps_around() {
        let items = vec![
            WorktreeView::test_item("wt1", WorktreeState::Active),
            WorktreeView::test_item("wt2", WorktreeState::Suspended),
        ];
        let mut view = WorktreeView::new(items);

        view.selected_index = 0;
        view.select_previous();
        assert_eq!(view.selected_index, 1);
        view.select_previous();
        assert_eq!(view.selected_index, 0);
    }

    #[test]
    fn worktree_view_selected_item_returns_correct_item() {
        let items = vec![
            WorktreeView::test_item("wt1", WorktreeState::Active),
            WorktreeView::test_item("wt2", WorktreeState::Suspended),
        ];
        let mut view = WorktreeView::new(items);

        assert!(view.selected_item().is_some());
        assert_eq!(view.selected_item().unwrap().name, "wt1");

        view.selected_index = 1;
        assert_eq!(view.selected_item().unwrap().name, "wt2");
    }

    #[test]
    fn worktree_view_empty_select_next_does_nothing() {
        let mut view = WorktreeView::new(Vec::new());
        view.select_next();
        assert_eq!(view.selected_index, 0);
    }

    #[test]
    fn worktree_view_default_creates_empty_view() {
        let view = WorktreeView::default();
        assert!(view.items.is_empty());
    }

    // ── Adversarial: Send + Sync ──

    #[test]
    fn worktree_view_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<WorktreeView>();
    }

    #[test]
    fn worktree_view_is_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<WorktreeView>();
    }

    // ── Adversarial: single item navigation ──

    #[test]
    fn single_item_navigation_stays_at_zero() {
        let items = vec![WorktreeView::test_item("wt1", WorktreeState::Active)];
        let mut view = WorktreeView::new(items);
        for _ in 0..100 {
            view.select_next();
        }
        assert_eq!(view.selected_index, 0);
        for _ in 0..100 {
            view.select_previous();
        }
        assert_eq!(view.selected_index, 0);
        assert_eq!(view.selected_item().unwrap().name, "wt1");
    }

    // ── Adversarial: re-emptied view navigation ──

    #[test]
    fn re_emptied_view_navigation_no_panic() {
        let items = vec![
            WorktreeView::test_item("wt1", WorktreeState::Active),
            WorktreeView::test_item("wt2", WorktreeState::Suspended),
        ];
        let mut view = WorktreeView::new(items);
        view = view.with_items(Vec::new());
        view.select_next();
        view.select_previous();
        assert_eq!(view.selected_index, 0);
        assert!(view.selected_item().is_none());
    }

    // ── Adversarial: render empty view ──

    #[test]
    fn render_empty_view_no_panic() {
        use ratatui::{backend::TestBackend, Terminal};
        let view = WorktreeView::default();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("terminal creation");
        terminal.draw(|f| {
            let area = f.area();
            view.render(f, area);
        }).expect("draw");
    }

    // ── Adversarial: render with items ──

    #[test]
    fn render_with_items_no_panic() {
        use ratatui::{backend::TestBackend, Terminal};
        let items = vec![
            WorktreeView::test_item("wt1", WorktreeState::Active),
            WorktreeView::test_item("wt2", WorktreeState::Suspended),
        ];
        let view = WorktreeView::new(items);
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("terminal creation");
        terminal.draw(|f| {
            let area = f.area();
            view.render(f, area);
        }).expect("draw");
    }

    // ── Adversarial: render tiny area ──

    #[test]
    fn render_tiny_area_no_panic() {
        use ratatui::{backend::TestBackend, Terminal};
        let view = WorktreeView::default();
        let backend = TestBackend::new(1, 1);
        let mut terminal = Terminal::new(backend).expect("terminal creation");
        terminal.draw(|f| {
            let area = f.area();
            view.render(f, area);
        }).expect("draw");
    }

    // ── Adversarial: render zero area ──

    #[test]
    fn render_zero_area_no_panic() {
        use ratatui::{backend::TestBackend, Terminal};
        let view = WorktreeView::default();
        let backend = TestBackend::new(0, 0);
        let mut terminal = Terminal::new(backend).expect("terminal creation");
        terminal.draw(|f| {
            let area = f.area();
            view.render(f, area);
        }).expect("draw");
    }

    // ── Adversarial: render with active item ──

    #[test]
    fn render_with_active_item_no_panic() {
        use ratatui::{backend::TestBackend, Terminal};
        let mut item = WorktreeView::test_item("active-wt", WorktreeState::Active);
        item.is_active = true;
        let view = WorktreeView::new(vec![item]);
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("terminal creation");
        terminal.draw(|f| {
            let area = f.area();
            view.render(f, area);
        }).expect("draw");
    }

    // ── Adversarial: Debug format ──

    #[test]
    fn worktree_view_debug_format() {
        let view = WorktreeView::default();
        let debug = format!("{:?}", view);
        assert!(debug.contains("WorktreeView"));
    }
}
