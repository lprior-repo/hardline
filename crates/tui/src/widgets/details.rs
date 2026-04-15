//! Details view widget - Shows branch/PR details panel
//!
//! Renders a detail panel with branch info, PR status, and metadata.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// PR status for the details view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrStatus {
    /// PR is open and pending review.
    Open,
    /// PR has been approved.
    Approved,
    /// PR checks are passing.
    ChecksPassing,
    /// PR has merge conflicts.
    Conflicted,
    /// PR has been merged.
    Merged,
    /// PR was closed without merging.
    Closed,
}

impl PrStatus {
    /// Returns the display color.
    pub const fn color(self) -> Color {
        match self {
            Self::Open => Color::Yellow,
            Self::Approved => Color::Green,
            Self::ChecksPassing => Color::Green,
            Self::Conflicted => Color::Red,
            Self::Merged => Color::Magenta,
            Self::Closed => Color::DarkGray,
        }
    }

    /// Returns the display label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Open => "OPEN",
            Self::Approved => "APPROVED",
            Self::ChecksPassing => "PASSING",
            Self::Conflicted => "CONFLICT",
            Self::Merged => "MERGED",
            Self::Closed => "CLOSED",
        }
    }
}

/// Branch detail information.
#[derive(Debug, Clone)]
pub struct BranchDetail {
    /// Branch name.
    pub name: String,
    /// Parent branch.
    pub parent: Option<String>,
    /// Short commit hash.
    pub commit: String,
    /// Author of the last commit.
    pub author: String,
    /// Commit message summary.
    pub message: String,
    /// PR status, if a PR exists.
    pub pr_status: Option<PrStatus>,
    /// PR number, if a PR exists.
    pub pr_number: Option<u32>,
}

/// The details view widget.
pub struct DetailsViewWidget {
    detail: Option<BranchDetail>,
    title: String,
}

impl DetailsViewWidget {
    /// Create a new empty details view.
    pub fn new() -> Self {
        Self {
            detail: None,
            title: "Details".to_string(),
        }
    }

    /// Set the branch detail to display.
    pub fn with_detail(mut self, detail: BranchDetail) -> Self {
        self.detail = Some(detail);
        self
    }

    /// Set the widget title.
    pub fn with_title(mut self, title: &str) -> Self {
        self.title = title.to_string();
        self
    }

    /// Render the widget into the frame at the given area.
    pub fn render(self, f: &mut Frame, area: Rect) {
        let lines = match &self.detail {
            Some(detail) => self.render_detail(detail),
            None => vec![Line::from(Span::styled(
                "  Select a branch to view details",
                Style::default().fg(Color::DarkGray),
            ))],
        };

        let paragraph = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(self.title.as_str()),
        );

        f.render_widget(paragraph, area);
    }

    fn render_detail(&self, detail: &BranchDetail) -> Vec<Line<'static>> {
        let mut lines = Vec::new();

        // Branch name header
        lines.push(Line::from(vec![
            Span::styled(" Branch: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                detail.name.clone(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        // Parent
        if let Some(parent) = &detail.parent {
            lines.push(Line::from(vec![
                Span::styled(" Parent: ", Style::default().fg(Color::DarkGray)),
                Span::styled(parent.clone(), Style::default().fg(Color::White)),
            ]));
        }

        // Commit
        lines.push(Line::from(vec![
            Span::styled(" Commit: ", Style::default().fg(Color::DarkGray)),
            Span::styled(detail.commit.clone(), Style::default().fg(Color::Yellow)),
            Span::styled(
                format!(" ({})", detail.author),
                Style::default().fg(Color::DarkGray),
            ),
        ]));

        // Message
        lines.push(Line::from(vec![
            Span::styled("   Msg: ", Style::default().fg(Color::DarkGray)),
            Span::styled(detail.message.clone(), Style::default().fg(Color::White)),
        ]));

        // PR status
        if let (Some(pr_num), Some(pr_status)) = (detail.pr_number, detail.pr_status) {
            lines.push(Line::from(vec![
                Span::styled("    PR: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("#{}", pr_num),
                    Style::default().fg(Color::Blue),
                ),
                Span::raw(" "),
                Span::styled(
                    pr_status.label().to_string(),
                    Style::default()
                        .fg(pr_status.color())
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
        }

        lines
    }
}

impl Default for DetailsViewWidget {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_detail() -> BranchDetail {
        BranchDetail {
            name: "feature-auth".to_string(),
            parent: Some("main".to_string()),
            commit: "abc1234".to_string(),
            author: "alice".to_string(),
            message: "Add authentication module".to_string(),
            pr_status: Some(PrStatus::Approved),
            pr_number: Some(42),
        }
    }

    #[test]
    fn details_view_new_is_empty() {
        let widget = DetailsViewWidget::new();
        assert!(widget.detail.is_none());
    }

    #[test]
    fn details_view_default_is_new() {
        let widget = DetailsViewWidget::default();
        assert!(widget.detail.is_none());
    }

    #[test]
    fn details_view_with_detail() {
        let detail = make_detail();
        let widget = DetailsViewWidget::new().with_detail(detail);
        assert!(widget.detail.is_some());
    }

    #[test]
    fn details_view_with_title() {
        let widget = DetailsViewWidget::new().with_title("Branch Info");
        assert_eq!(widget.title, "Branch Info");
    }

    #[test]
    fn pr_status_color_mapping() {
        assert_eq!(PrStatus::Open.color(), Color::Yellow);
        assert_eq!(PrStatus::Approved.color(), Color::Green);
        assert_eq!(PrStatus::ChecksPassing.color(), Color::Green);
        assert_eq!(PrStatus::Conflicted.color(), Color::Red);
        assert_eq!(PrStatus::Merged.color(), Color::Magenta);
        assert_eq!(PrStatus::Closed.color(), Color::DarkGray);
    }

    #[test]
    fn pr_status_labels() {
        assert_eq!(PrStatus::Open.label(), "OPEN");
        assert_eq!(PrStatus::Approved.label(), "APPROVED");
        assert_eq!(PrStatus::ChecksPassing.label(), "PASSING");
        assert_eq!(PrStatus::Conflicted.label(), "CONFLICT");
        assert_eq!(PrStatus::Merged.label(), "MERGED");
        assert_eq!(PrStatus::Closed.label(), "CLOSED");
    }

    #[test]
    fn branch_detail_fields() {
        let detail = make_detail();
        assert_eq!(detail.name, "feature-auth");
        assert_eq!(detail.parent, Some("main".to_string()));
        assert_eq!(detail.commit, "abc1234");
        assert_eq!(detail.author, "alice");
        assert_eq!(detail.message, "Add authentication module");
        assert_eq!(detail.pr_status, Some(PrStatus::Approved));
        assert_eq!(detail.pr_number, Some(42));
    }

    #[test]
    fn branch_detail_no_pr() {
        let detail = BranchDetail {
            name: "local-branch".to_string(),
            parent: None,
            commit: "def5678".to_string(),
            author: "bob".to_string(),
            message: "WIP".to_string(),
            pr_status: None,
            pr_number: None,
        };
        assert!(detail.parent.is_none());
        assert!(detail.pr_status.is_none());
        assert!(detail.pr_number.is_none());
    }

    #[test]
    fn details_view_builder_pattern() {
        let widget = DetailsViewWidget::new()
            .with_title("Info")
            .with_detail(make_detail());
        assert_eq!(widget.title, "Info");
        assert!(widget.detail.is_some());
    }
}
