use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{App, Mode};

pub fn draw(frame: &mut Frame, app: &App) {
    let size = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // status bar
            Constraint::Min(3),    // results list
            Constraint::Length(1), // search/command line
        ])
        .split(size);

    draw_status(frame, chunks[0], app);
    draw_results(frame, chunks[1], app);
    draw_cmdline(frame, chunks[2], app);
}

fn draw_status(frame: &mut Frame, area: Rect, app: &App) {
    let text = if app.status.is_empty() {
        format!(
            " rsearch — {} docs indexed  |  '/' search   j/k move   Enter open   r reindex   q quit",
            app.index.live_doc_count()
        )
    } else {
        format!(" {}", app.status)
    };
    let p = Paragraph::new(text).style(Style::default().fg(Color::Black).bg(Color::Gray));
    frame.render_widget(p, area);
}

fn draw_results(frame: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app
        .results
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let selected = i == app.selected;
            let path_style = if selected {
                Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Cyan)
            };
            let line1 = Line::from(vec![
                Span::styled(format!("{:>6.2}  ", r.score), Style::default().fg(Color::Yellow)),
                Span::styled(r.path.clone(), path_style),
            ]);
            let line2 = Line::from(Span::styled(
                format!("        {}", r.snippet),
                Style::default().fg(Color::DarkGray),
            ));
            ListItem::new(vec![line1, line2])
        })
        .collect();

    let title = if app.results.is_empty() {
        " results — press / to search ".to_string()
    } else {
        format!(" results ({}) ", app.results.len())
    };

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(title));
    frame.render_widget(list, area);
}

fn draw_cmdline(frame: &mut Frame, area: Rect, app: &App) {
    let text = match app.mode {
        Mode::Search => format!("/{}", app.query),
        Mode::Normal => String::new(),
    };
    let p = Paragraph::new(text).style(Style::default().fg(Color::White));
    frame.render_widget(p, area);
}
