use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Style,
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::TuiApp;

/// Calculate the layout chunks for the TUI layout
fn calculate_layout_chunks(area: ratatui::layout::Rect) -> Vec<ratatui::layout::Rect> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area)
        .to_vec()
}

/// Construct the title paragraph widget
fn build_title_widget() -> Paragraph<'static> {
    Paragraph::new("SCP Stack TUI")
        .style(Style::default())
        .block(Block::default().borders(Borders::ALL).title("Stax"))
}

/// Construct the stack list widget
fn build_stack_list_widget() -> List<'static> {
    List::new(vec![ListItem::new("Stack branches will appear here")])
        .block(Block::default().borders(Borders::ALL).title("Stack"))
}

/// Construct the status paragraph widget
fn build_status_widget() -> Paragraph<'static> {
    Paragraph::new("Press q to quit")
        .style(Style::default())
        .block(Block::default().borders(Borders::ALL))
}

pub fn render(f: &mut Frame, _app: &mut TuiApp) {
    let chunks = calculate_layout_chunks(f.area());

    f.render_widget(build_title_widget(), chunks[0]);
    f.render_widget(build_stack_list_widget(), chunks[1]);
    f.render_widget(build_status_widget(), chunks[2]);
}
