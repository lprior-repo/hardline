use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Widget},
};
use scp_core::{Agent, AgentActivity, AgentStatus};

/// Display data for a single agent row in the AgentsView.
///
/// Decouples the widget from the domain `Agent` type, allowing the view
/// to render agents from any source (registry, CLI, test fixtures).
#[derive(Debug, Clone)]
pub struct AgentEntry {
    pub name: String,
    pub status: AgentStatus,
    pub activity: AgentActivity,
    pub actions_count: u64,
}

impl AgentEntry {
    #[must_use]
    pub fn from_agent(agent: &Agent) -> Self {
        Self {
            name: agent.id.to_string(),
            status: agent.status(),
            activity: agent.activity.clone(),
            actions_count: agent.actions_count,
        }
    }
}

/// Widget that displays polecat agent status in a TUI panel.
///
/// Shows agent name, status indicator, current work, and action count.
/// Status is color-coded: green for active, yellow for idle, red for stale.
#[derive(Debug, Clone)]
pub struct AgentsView {
    pub agents: Vec<AgentEntry>,
    pub selected_index: Option<usize>,
}

impl AgentsView {
    #[must_use]
    pub fn new(agents: Vec<AgentEntry>) -> Self {
        Self {
            agents,
            selected_index: None,
        }
    }

    #[must_use]
    pub fn with_selection(mut self, index: Option<usize>) -> Self {
        self.selected_index = index;
        self
    }

    fn status_indicator(status: AgentStatus) -> (&'static str, Color) {
        match status {
            AgentStatus::Active => ("●", Color::Green),
            AgentStatus::Stale => ("●", Color::Red),
        }
    }

    fn activity_text(activity: &AgentActivity) -> (String, Color) {
        match activity {
            AgentActivity::Idle => ("idle".to_string(), Color::DarkGray),
            AgentActivity::Working { session, command } => {
                (format!("{} ({})", session, command), Color::Cyan)
            }
        }
    }
}

impl Widget for AgentsView {
    fn render(self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Agents")
            .title_style(Style::default());

        let inner_area = block.inner(area);
        block.render(area, buf);

        if self.agents.is_empty() {
            let no_agents = ListItem::new(Line::from(Span::styled(
                "No agents registered",
                Style::default().fg(Color::DarkGray),
            )));
            let list = List::new(vec![no_agents]).style(Style::default());
            list.render(inner_area, buf);
            return;
        }

        let items = build_agent_list_items(&self.agents, self.selected_index);
        let list = List::new(items).style(Style::default());
        list.render(inner_area, buf);
    }
}

/// Build the list items for each agent in the view.
fn build_agent_list_items(agents: &[AgentEntry], selected_index: Option<usize>) -> Vec<ListItem> {
    agents
        .iter()
        .enumerate()
        .map(|(idx, agent)| {
            let mut spans: Vec<Span> = Vec::new();

            let (indicator, indicator_color) = AgentsView::status_indicator(agent.status);
            spans.push(Span::styled(
                format!("{} ", indicator),
                Style::default().fg(indicator_color),
            ));

            spans.push(Span::styled(
                format!("{:<16}", agent.name),
                Style::default().fg(Color::White),
            ));

            let (activity_str, activity_color) = AgentsView::activity_text(&agent.activity);
            spans.push(Span::styled(
                format!("{:<24}", activity_str),
                Style::default().fg(activity_color),
            ));

            spans.push(Span::styled(
                format!("{} actions", agent.actions_count),
                Style::default().fg(Color::DarkGray),
            ));

            if selected_index == Some(idx) {
                let last = spans.len() - 1;
                spans[last] = spans[last]
                    .clone()
                    .style(Style::default().bg(Color::Blue));
            }

            ListItem::new(Line::from(spans))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use scp_core::AgentId;

    fn make_entry(name: &str, status: AgentStatus, activity: AgentActivity, count: u64) -> AgentEntry {
        AgentEntry {
            name: name.to_string(),
            status,
            activity,
            actions_count: count,
        }
    }

    fn active_working_entry(name: &str, session: &str, cmd: &str, count: u64) -> AgentEntry {
        make_entry(name, AgentStatus::Active, AgentActivity::Working {
            session: session.to_string(),
            command: cmd.to_string(),
        }, count)
    }

    fn active_idle_entry(name: &str, count: u64) -> AgentEntry {
        make_entry(name, AgentStatus::Active, AgentActivity::Idle, count)
    }

    fn stale_entry(name: &str, count: u64) -> AgentEntry {
        make_entry(name, AgentStatus::Stale, AgentActivity::Idle, count)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Construction
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn new_with_empty_agents() {
        let view = AgentsView::new(Vec::new());
        assert!(view.agents.is_empty());
        assert_eq!(view.selected_index, None);
    }

    #[test]
    fn new_with_agents() {
        let entries = vec![active_idle_entry("alpha", 0)];
        let view = AgentsView::new(entries);
        assert_eq!(view.agents.len(), 1);
    }

    #[test]
    fn with_selection() {
        let entries = vec![active_idle_entry("a", 0), active_idle_entry("b", 0)];
        let view = AgentsView::new(entries).with_selection(Some(1));
        assert_eq!(view.selected_index, Some(1));
    }

    #[test]
    fn with_selection_none_clears() {
        let entries = vec![active_idle_entry("a", 0)];
        let view = AgentsView::new(entries).with_selection(None);
        assert_eq!(view.selected_index, None);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // AgentEntry::from_agent
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn agent_entry_from_agent_idle() {
        let agent = Agent::new(AgentId::new("test-idle"));
        let entry = AgentEntry::from_agent(&agent);
        assert_eq!(entry.name, "test-idle");
        assert_eq!(entry.status, AgentStatus::Active);
        assert!(!entry.activity.is_working());
        assert_eq!(entry.actions_count, 0);
    }

    #[test]
    fn agent_entry_from_agent_working() {
        let mut agent = Agent::new(AgentId::new("test-working"));
        agent.start_work("sess-1", "build");
        let entry = AgentEntry::from_agent(&agent);
        assert_eq!(entry.name, "test-working");
        assert!(entry.activity.is_working());
        assert_eq!(entry.actions_count, 1);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // status_indicator
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn status_indicator_active() {
        let (sym, color) = AgentsView::status_indicator(AgentStatus::Active);
        assert_eq!(sym, "●");
        assert_eq!(color, Color::Green);
    }

    #[test]
    fn status_indicator_stale() {
        let (sym, color) = AgentsView::status_indicator(AgentStatus::Stale);
        assert_eq!(sym, "●");
        assert_eq!(color, Color::Red);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // activity_text
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn activity_text_idle() {
        let (text, color) = AgentsView::activity_text(&AgentActivity::Idle);
        assert_eq!(text, "idle");
        assert_eq!(color, Color::DarkGray);
    }

    #[test]
    fn activity_text_working() {
        let activity = AgentActivity::Working {
            session: "sess-42".to_string(),
            command: "test".to_string(),
        };
        let (text, color) = AgentsView::activity_text(&activity);
        assert_eq!(text, "sess-42 (test)");
        assert_eq!(color, Color::Cyan);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Trait implementations
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn agents_view_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<AgentsView>();
    }

    #[test]
    fn agents_view_is_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<AgentsView>();
    }

    #[test]
    fn agents_view_is_clone() {
        let view = AgentsView::new(vec![active_idle_entry("x", 0)]);
        let cloned = view.clone();
        assert_eq!(cloned.agents.len(), 1);
    }

    #[test]
    fn agents_view_debug() {
        let view = AgentsView::new(Vec::new());
        let debug = format!("{:?}", view);
        assert!(debug.contains("AgentsView"));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Rendering (via TestBackend)
    // ═══════════════════════════════════════════════════════════════════════════

    use ratatui::{backend::TestBackend, Terminal};

    fn render_to_string(view: AgentsView, width: u16, height: u16) -> String {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| {
            f.render_widget(view, f.area());
        }).unwrap();
        let buf = terminal.backend().buffer().clone();
        let area = buf.area;
        let mut lines = Vec::new();
        for y in 0..area.height {
            let start = (y * area.width) as usize;
            let end = start + area.width as usize;
            let line: String = buf.content[start..end]
                .iter()
                .map(|c| c.symbol())
                .collect();
            lines.push(line.trim_end().to_string());
        }
        lines.join("\n")
    }

    #[test]
    fn render_empty_shows_placeholder() {
        let view = AgentsView::new(Vec::new());
        let output = render_to_string(view, 60, 8);
        assert!(output.contains("No agents registered"));
        assert!(output.contains("Agents"));
    }

    #[test]
    fn render_single_active_agent() {
        let view = AgentsView::new(vec![active_idle_entry("alpha", 5)]);
        let output = render_to_string(view, 80, 8);
        assert!(output.contains("alpha"));
        assert!(output.contains("idle"));
        assert!(output.contains("5 actions"));
    }

    #[test]
    fn render_working_agent_shows_session() {
        let view = AgentsView::new(vec![active_working_entry("beta", "sess-1", "build", 3)]);
        let output = render_to_string(view, 80, 8);
        assert!(output.contains("beta"));
        assert!(output.contains("sess-1"));
        assert!(output.contains("build"));
    }

    #[test]
    fn render_stale_agent() {
        let view = AgentsView::new(vec![stale_entry("ghost", 0)]);
        let output = render_to_string(view, 80, 8);
        assert!(output.contains("ghost"));
        assert!(output.contains("idle"));
    }

    #[test]
    fn render_multiple_agents() {
        let entries = vec![
            active_working_entry("alpha", "s1", "build", 10),
            active_idle_entry("beta", 3),
            stale_entry("gamma", 7),
        ];
        let view = AgentsView::new(entries);
        let output = render_to_string(view, 80, 10);
        assert!(output.contains("alpha"));
        assert!(output.contains("beta"));
        assert!(output.contains("gamma"));
    }

    #[test]
    fn render_selection_highlight() {
        let entries = vec![
            active_idle_entry("alpha", 0),
            active_idle_entry("beta", 0),
        ];
        let view = AgentsView::new(entries).with_selection(Some(1));
        let output = render_to_string(view, 80, 8);
        assert!(output.contains("alpha"));
        assert!(output.contains("beta"));
    }

    #[test]
    fn render_selection_out_of_bounds_does_not_panic() {
        let entries = vec![active_idle_entry("solo", 0)];
        let view = AgentsView::new(entries).with_selection(Some(999));
        let output = render_to_string(view, 80, 8);
        assert!(output.contains("solo"));
    }

    #[test]
    fn render_zero_width_area_does_not_panic() {
        let view = AgentsView::new(vec![active_idle_entry("x", 0)]);
        let output = render_to_string(view, 1, 1);
        // Just verify no panic
        let _ = output;
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Adversarial / edge cases
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn long_agent_name_does_not_panic() {
        let name = "a".repeat(200);
        let view = AgentsView::new(vec![active_idle_entry(&name, 0)]);
        let output = render_to_string(view, 80, 8);
        assert!(output.contains("a"));
    }

    #[test]
    fn empty_agent_name_renders() {
        let view = AgentsView::new(vec![active_idle_entry("", 0)]);
        let output = render_to_string(view, 80, 8);
        let _ = output;
    }

    #[test]
    fn unicode_agent_name_renders() {
        let view = AgentsView::new(vec![active_idle_entry("polecat-β", 0)]);
        let output = render_to_string(view, 80, 8);
        assert!(output.contains("polecat"));
    }

    #[test]
    fn many_agents_truncated_by_area() {
        let entries: Vec<AgentEntry> = (0..100)
            .map(|i| active_idle_entry(&format!("agent-{:03}", i), i as u64))
            .collect();
        let view = AgentsView::new(entries);
        let output = render_to_string(view, 80, 10);
        assert!(output.contains("agent-000"));
    }

    #[test]
    fn high_action_count_formats() {
        let view = AgentsView::new(vec![active_idle_entry("counter", u64::MAX)]);
        let output = render_to_string(view, 80, 8);
        assert!(output.contains(&u64::MAX.to_string()));
    }
}
