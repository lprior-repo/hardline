//! Diff view widget - Renders file diffs with syntax highlighting
//!
//! Displays unified diff output with added/removed line coloring.

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

/// A single line in a diff.
#[derive(Debug, Clone)]
pub enum DiffLine {
    /// Context line (unchanged).
    Context(String),
    /// Added line.
    Added(String),
    /// Removed line.
    Removed(String),
    /// File header (e.g., "--- a/file.rs").
    Header(String),
    /// Hunk header (e.g., "@@ -1,3 +1,4 @@").
    HunkHeader(String),
}

impl DiffLine {
    /// Returns the styled spans for rendering this line.
    pub fn to_spans(&self) -> Line<'static> {
        match self {
            Self::Context(content) => Line::from(Span::styled(
                format!(" {content}"),
                Style::default().fg(Color::White),
            )),
            Self::Added(content) => Line::from(Span::styled(
                format!("+{content}"),
                Style::default().fg(Color::Green),
            )),
            Self::Removed(content) => Line::from(Span::styled(
                format!("-{content}"),
                Style::default().fg(Color::Red),
            )),
            Self::Header(content) => Line::from(Span::styled(
                content.clone(),
                Style::default().fg(Color::DarkGray),
            )),
            Self::HunkHeader(content) => Line::from(Span::styled(
                format!(" {content}"),
                Style::default().fg(Color::Blue),
            )),
        }
    }
}

/// The diff view widget.
pub struct DiffViewWidget {
    lines: Vec<DiffLine>,
    title: String,
}

impl DiffViewWidget {
    /// Create a new empty diff view.
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            title: "Diff".to_string(),
        }
    }

    /// Set the diff lines to display.
    pub fn with_lines(mut self, lines: Vec<DiffLine>) -> Self {
        self.lines = lines;
        self
    }

    /// Set the widget title.
    pub fn with_title(mut self, title: &str) -> Self {
        self.title = title.to_string();
        self
    }

    /// Render the widget into the frame at the given area.
    pub fn render(self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .lines
            .iter()
            .map(|line| ListItem::new(line.to_spans()))
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(self.title.as_str()),
        );

        f.render_widget(list, area);
    }
}

impl Default for DiffViewWidget {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_view_new_is_empty() {
        let widget = DiffViewWidget::new();
        assert!(widget.lines.is_empty());
    }

    #[test]
    fn diff_view_default_is_new() {
        let widget = DiffViewWidget::default();
        assert!(widget.lines.is_empty());
    }

    #[test]
    fn diff_view_with_lines() {
        let lines = vec![
            DiffLine::Header("--- a/file.rs".to_string()),
            DiffLine::Added("new line".to_string()),
            DiffLine::Removed("old line".to_string()),
        ];
        let widget = DiffViewWidget::new().with_lines(lines);
        assert_eq!(widget.lines.len(), 3);
    }

    #[test]
    fn diff_view_with_title() {
        let widget = DiffViewWidget::new().with_title("Changes");
        assert_eq!(widget.title, "Changes");
    }

    #[test]
    fn diff_line_context_to_spans() {
        let line = DiffLine::Context("unchanged".to_string());
        let spans = line.to_spans();
        assert_eq!(spans.spans.len(), 1);
    }

    #[test]
    fn diff_line_added_to_spans() {
        let line = DiffLine::Added("new code".to_string());
        let spans = line.to_spans();
        assert_eq!(spans.spans.len(), 1);
    }

    #[test]
    fn diff_line_removed_to_spans() {
        let line = DiffLine::Removed("old code".to_string());
        let spans = line.to_spans();
        assert_eq!(spans.spans.len(), 1);
    }

    #[test]
    fn diff_line_header_to_spans() {
        let line = DiffLine::Header("--- a/file.rs".to_string());
        let spans = line.to_spans();
        assert_eq!(spans.spans.len(), 1);
    }

    #[test]
    fn diff_line_hunk_header_to_spans() {
        let line = DiffLine::HunkHeader("@@ -1,3 +1,4 @@".to_string());
        let spans = line.to_spans();
        assert_eq!(spans.spans.len(), 1);
    }

    #[test]
    fn diff_view_builder_pattern() {
        let widget = DiffViewWidget::new()
            .with_title("PR #42")
            .with_lines(vec![
                DiffLine::Added("feat: new function".to_string()),
            ]);
        assert_eq!(widget.title, "PR #42");
        assert_eq!(widget.lines.len(), 1);
    }
}
