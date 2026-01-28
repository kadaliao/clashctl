use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{AppState, Mode};
use crate::clash::HumanRoute;

pub fn render(f: &mut Frame, area: Rect, state: &AppState, selected_index: usize) {
    let constraints = if state.status_message.is_some() {
        vec![
            Constraint::Length(3),  // Title
            Constraint::Length(3),  // Status message
            Constraint::Min(0),     // Route list
            Constraint::Length(3),  // Help
        ]
    } else {
        vec![
            Constraint::Length(3),  // Title
            Constraint::Min(0),     // Route list
            Constraint::Length(3),  // Help
        ]
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let mut chunk_idx = 0;
    render_title(f, chunks[chunk_idx], state.mode);
    chunk_idx += 1;

    if let Some(msg) = &state.status_message {
        render_status(f, chunks[chunk_idx], msg);
        chunk_idx += 1;
    }

    render_routes(f, chunks[chunk_idx], state, selected_index);
    chunk_idx += 1;

    render_help(f, chunks[chunk_idx], state.mode);
}

fn render_title(f: &mut Frame, area: Rect, mode: Mode) {
    let title_text = format!("Route Management [{}]", mode.as_str());
    let title = Paragraph::new(title_text)
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, area);
}

fn render_status(f: &mut Frame, area: Rect, message: &str) {
    let status = Paragraph::new(message)
        .style(Style::default().fg(Color::Yellow))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(status, area);
}

fn render_routes(f: &mut Frame, area: Rect, state: &AppState, selected_index: usize) {
    let routes = HumanRoute::from_proxies(&state.clash_state.proxies, state.mode);

    if routes.is_empty() {
        let empty = Paragraph::new("No routes available")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Routes"));
        f.render_widget(empty, area);
        return;
    }

    let items: Vec<ListItem> = routes
        .iter()
        .enumerate()
        .map(|(i, route)| {
            let is_selected = i == selected_index;
            let display_name = route.display_name();
            let current_display = route.current_display();
            let node_count = format!(" ({} nodes)", route.node_count);

            let content = if is_selected {
                Line::from(vec![
                    Span::styled("> ", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        display_name,
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" → "),
                    Span::styled(current_display, Style::default().fg(Color::Green)),
                    Span::raw(node_count),
                ])
            } else {
                Line::from(vec![
                    Span::raw("  "),
                    Span::raw(display_name),
                    Span::raw(" → "),
                    Span::styled(current_display, Style::default().fg(Color::Gray)),
                    Span::styled(node_count, Style::default().fg(Color::DarkGray)),
                ])
            };

            ListItem::new(content)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(format!(
            "Routes ({}/{})",
            selected_index + 1,
            routes.len()
        )));

    f.render_widget(list, area);
}

fn render_help(f: &mut Frame, area: Rect, mode: Mode) {
    let mode_hint = if mode == Mode::Simple {
        vec![
            Span::styled("Ctrl+E", Style::default().fg(Color::Yellow)),
            Span::raw(" Expert Mode  "),
        ]
    } else {
        vec![
            Span::styled("Ctrl+E", Style::default().fg(Color::Yellow)),
            Span::raw(" Simple Mode  "),
        ]
    };

    let mut help_spans = mode_hint;
    help_spans.extend(vec![
        Span::styled("↑↓", Style::default().fg(Color::Yellow)),
        Span::raw(" Navigate  "),
        Span::styled("Enter", Style::default().fg(Color::Yellow)),
        Span::raw(" Switch  "),
        Span::styled("t", Style::default().fg(Color::Yellow)),
        Span::raw(" Test  "),
        Span::styled("h", Style::default().fg(Color::Yellow)),
        Span::raw(" Home  "),
        Span::styled("q", Style::default().fg(Color::Yellow)),
        Span::raw(" Quit"),
    ]);

    let help = Paragraph::new(Line::from(help_spans))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));

    f.render_widget(help, area);
}
