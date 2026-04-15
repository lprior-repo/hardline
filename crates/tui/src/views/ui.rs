use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{FocusedPane, TuiApp};

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
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Stack Tree")
        .title_style(if app.focused_pane == FocusedPane::Stack {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        });

    let stack_items = vec![
        ListItem::new(Line::from(vec![
            Span::raw("● "),
            Span::styled("main", Style::default().fg(Color::Green)),
        ])),
        ListItem::new(Line::from(vec![
            Span::raw("├─ "),
            Span::styled("feature/auth", Style::default().fg(Color::Blue)),
        ])),
        ListItem::new(Line::from(vec![
            Span::raw("│  "),
            Span::styled("└─ ", Style::default().fg(Color::Blue)),
            Span::styled("fix/bug-123", Style::default().fg(Color::Yellow)),
        ])),
        ListItem::new(Line::from(vec![
            Span::raw("│     "),
            Span::styled("└─ ", Style::default().fg(Color::Yellow)),
            Span::styled("wip/temp", Style::default().fg(Color::Red)),
        ])),
    ];

    let list = List::new(stack_items).block(block).style(Style::default());

    f.render_widget(list, area);
}

fn render_diff_view(f: &mut Frame, _app: &mut TuiApp, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Diff Viewer")
        .title_style(if _app.focused_pane == FocusedPane::Diff {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        });

    let diff_content = vec![
        Line::from(vec![Span::styled(
            "diff --git a/src/lib.rs b/src/lib.rs",
            Style::default().fg(Color::Cyan),
        )]),
        Line::from(vec![Span::raw("index 1234567..89abcdef 100644")]),
        Line::from(vec![Span::raw("--- a/src/lib.rs")]),
        Line::from(vec![Span::raw("+++ b/src/lib.rs")]),
        Line::from(vec![Span::styled(
            "@@ -1,5 +1,6 @@",
            Style::default().fg(Color::Magenta),
        )]),
        Line::from(vec![Span::raw(" use crate::app;")]),
        Line::from(vec![Span::raw(" use crate::error;")]),
        Line::from(vec![Span::styled(
            "+use crate::views;",
            Style::default().fg(Color::LightGreen),
        )]),
        Line::from(vec![Span::raw(" ")]),
        Line::from(vec![Span::styled(
            "-fn old_function() {",
            Style::default().fg(Color::LightRed),
        )]),
        Line::from(vec![Span::styled(
            "+fn new_function() {",
            Style::default().fg(Color::LightGreen),
        )]),
        Line::from(vec![Span::raw("     // TODO: implement")]),
        Line::from(vec![Span::styled(
            "@@ -10,3 +11,4 @@",
            Style::default().fg(Color::Magenta),
        )]),
        Line::from(vec![Span::styled(
            "+    unimplemented!();",
            Style::default().fg(Color::LightGreen),
        )]),
        Line::from(vec![Span::raw(" }")]),
    ];

    let diff_para = Paragraph::new(diff_content).block(block).scroll((0, 0));

    f.render_widget(diff_para, area);
}
