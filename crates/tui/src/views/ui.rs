use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::Style,
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::TuiApp;

pub fn render(f: &mut Frame, _app: &mut TuiApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.area());

    let title = Paragraph::new("SCP Stack TUI")
        .style(Style::default())
        .block(Block::default().borders(Borders::ALL).title("Stax"));
    f.render_widget(title, chunks[0]);

    let list = List::new(vec![ListItem::new("Stack branches will appear here")])
        .block(Block::default().borders(Borders::ALL).title("Stack"));
    f.render_widget(list, chunks[1]);

    let status = Paragraph::new("Press q to quit")
        .style(Style::default())
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(status, chunks[2]);
}
