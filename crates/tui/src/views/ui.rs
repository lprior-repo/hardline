use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

<<<<<<< HEAD
use crate::app::{FocusedPane, TuiApp};
use crate::widgets::diff::DiffLine;
=======
use crate::app::{DiffLine, FocusedPane, TuiApp};
>>>>>>> polecat/epsilon
use crate::widgets::StackTreeWidget;

pub fn render(f: &mut Frame, app: &mut TuiApp) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);

    let title = Paragraph::new("SCP Stack TUI")
        .style(Style::default())
        .block(Block::default().borders(Borders::ALL).title("Stax"));
    f.render_widget(title, chunks[0]);

    let main_area = chunks[1];
    let split_ratio = if app.focused_pane == FocusedPane::Stack {
        0.35
    } else {
        0.40
    };
    let split_width = (main_area.width as f32 * split_ratio) as u16;
    let left_width = if app.focused_pane == FocusedPane::Stack {
        split_width
    } else {
        split_width.saturating_sub(1)
    };
    let right_width = main_area.width.saturating_sub(left_width);

    let left_rect = Rect::new(main_area.x, main_area.y, left_width, main_area.height);
    let right_rect = Rect::new(
        main_area.x + left_width,
        main_area.y,
        right_width,
        main_area.height,
    );

    render_stack_tree(f, app, left_rect);
    render_diff_view(f, app, right_rect);

    let status_msg = match app.focused_pane {
        FocusedPane::Stack => "Stack Tree | Tab: Switch panes | q: Quit",
        FocusedPane::Diff => "Diff View | Tab: Switch panes | Space: Stage hunk | q: Quit",
        FocusedPane::Worktrees => "Worktrees | Tab: Switch panes | q: Quit",
    };
    let status = Paragraph::new(status_msg)
        .style(Style::default())
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(status, chunks[2]);
}

fn render_stack_tree(f: &mut Frame, app: &mut TuiApp, area: Rect) {
    let _title_style = if app.focused_pane == FocusedPane::Stack {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let widget = StackTreeWidget::new(app.stack_branches.clone());
    f.render_widget(widget, area);
}

fn render_diff_view(f: &mut Frame, app: &mut TuiApp, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Diff Viewer")
        .title_style(if app.focused_pane == FocusedPane::Diff {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        });

<<<<<<< HEAD
    let diff_content = if app.diff_lines.is_empty() {
=======
    let diff_content = if _app.diff_lines.is_empty() {
>>>>>>> polecat/epsilon
        vec![Line::from(Span::styled(
            "No diff available",
            Style::default().fg(Color::DarkGray),
        ))]
    } else {
<<<<<<< HEAD
        app.diff_lines
            .iter()
            .map(|line| match line {
                DiffLine::Header(text) => {
                    Line::from(Span::styled(text.clone(), Style::default().fg(Color::Cyan)))
                }
                DiffLine::HunkHeader(text) => {
                    Line::from(Span::styled(text.clone(), Style::default().fg(Color::Magenta)))
                }
                DiffLine::Addition(text) => {
                    Line::from(Span::styled(text.clone(), Style::default().fg(Color::LightGreen)))
                }
                DiffLine::Deletion(text) => {
                    Line::from(Span::styled(text.clone(), Style::default().fg(Color::LightRed)))
                }
                DiffLine::Context(text) => Line::from(Span::raw(text.clone())),
=======
        _app
            .diff_lines
            .iter()
            .map(|line| match line {
                DiffLine::Header(s) => {
                    Line::from(Span::styled(s.clone(), Style::default().fg(Color::Cyan)))
                }
                DiffLine::Hunk(s) => {
                    Line::from(Span::styled(s.clone(), Style::default().fg(Color::Magenta)))
                }
                DiffLine::Context(s) => Line::from(Span::raw(s.clone())),
                DiffLine::Add(s) => {
                    Line::from(Span::styled(s.clone(), Style::default().fg(Color::LightGreen)))
                }
                DiffLine::Remove(s) => {
                    Line::from(Span::styled(s.clone(), Style::default().fg(Color::LightRed)))
                }
>>>>>>> polecat/epsilon
            })
            .collect()
    };

    let diff_para = Paragraph::new(diff_content).block(block).scroll((0, 0));

    f.render_widget(diff_para, area);
}
