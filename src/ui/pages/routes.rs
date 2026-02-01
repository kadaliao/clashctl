use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{AppState, Mode};
use crate::clash::HumanRoute;
use crate::config::{AppConfig, Preset};

pub fn render(
    f: &mut Frame,
    area: Rect,
    state: &AppState,
    config: &AppConfig,
    selected_index: usize,
) {
    render_normal_view(f, area, state, config, selected_index);
}

pub fn render_with_nodes(
    f: &mut Frame,
    area: Rect,
    state: &AppState,
    config: &AppConfig,
    route_index: usize,
    node_index: usize,
) {
    render_expanded_view(f, area, state, config, route_index, node_index);
}

fn render_normal_view(
    f: &mut Frame,
    area: Rect,
    state: &AppState,
    _config: &AppConfig,
    selected_index: usize,
) {
    let constraints = if state.status_message.is_some() {
        vec![
            Constraint::Length(3), // Title
            Constraint::Length(3), // Status message
            Constraint::Min(0),    // Route list
            Constraint::Length(3), // Help
        ]
    } else {
        vec![
            Constraint::Length(3), // Title
            Constraint::Min(0),    // Route list
            Constraint::Length(3), // Help
        ]
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let mut chunk_idx = 0;
    render_title(f, chunks[chunk_idx], state.mode, &state.preset, false);
    chunk_idx += 1;

    if let Some(msg) = &state.status_message {
        render_status(f, chunks[chunk_idx], msg);
        chunk_idx += 1;
    }

    render_routes(f, chunks[chunk_idx], state, selected_index);
    chunk_idx += 1;

    render_help(f, chunks[chunk_idx], state.mode, &state.preset, false);
}

fn render_expanded_view(
    f: &mut Frame,
    area: Rect,
    state: &AppState,
    config: &AppConfig,
    route_index: usize,
    node_index: usize,
) {
    let constraints = if state.status_message.is_some() {
        vec![
            Constraint::Length(3), // Title
            Constraint::Length(3), // Status message
            Constraint::Min(0),    // Node list
            Constraint::Length(4), // Help
        ]
    } else {
        vec![
            Constraint::Length(3), // Title
            Constraint::Min(0),    // Node list
            Constraint::Length(4), // Help
        ]
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let mut chunk_idx = 0;
    render_title(f, chunks[chunk_idx], state.mode, &state.preset, true);
    chunk_idx += 1;

    if let Some(msg) = &state.status_message {
        render_status(f, chunks[chunk_idx], msg);
        chunk_idx += 1;
    }

    render_nodes(f, chunks[chunk_idx], state, config, route_index, node_index);
    chunk_idx += 1;

    render_help(f, chunks[chunk_idx], state.mode, &state.preset, true);
}

fn render_title(f: &mut Frame, area: Rect, _mode: Mode, preset: &Preset, expanded: bool) {
    let title_text = if expanded {
        format!("Route Management [{}] - Node Selection", preset.name())
    } else {
        format!("Route Management [{}]", preset.name())
    };

    let title = Paragraph::new(title_text)
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
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
                    Span::styled(
                        " [Enter to view nodes]",
                        Style::default().fg(Color::DarkGray),
                    ),
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

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(format!(
        "Routes ({}/{}) - Press Enter to view nodes",
        selected_index + 1,
        routes.len()
    )));

    f.render_widget(list, area);
}

fn render_nodes(
    f: &mut Frame,
    area: Rect,
    state: &AppState,
    config: &AppConfig,
    route_index: usize,
    node_index: usize,
) {
    let routes = HumanRoute::from_proxies(&state.clash_state.proxies, state.mode);

    if route_index >= routes.len() {
        let empty = Paragraph::new("No routes available")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(empty, area);
        return;
    }

    let route = &routes[route_index];
    let nodes = &route.all_nodes;

    if nodes.is_empty() {
        let empty = Paragraph::new("No nodes available")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(empty, area);
        return;
    }

    let visible_items = area.height.saturating_sub(2).max(1) as usize;
    let selected_index = node_index.min(nodes.len().saturating_sub(1));
    let mut start_index = 0usize;
    if nodes.len() > visible_items {
        if selected_index >= visible_items {
            start_index = selected_index + 1 - visible_items;
        }
        let max_start = nodes.len().saturating_sub(visible_items);
        if start_index > max_start {
            start_index = max_start;
        }
    }
    let end_index = (start_index + visible_items).min(nodes.len());

    let items: Vec<ListItem> = nodes
        .iter()
        .enumerate()
        .skip(start_index)
        .take(end_index.saturating_sub(start_index))
        .map(|(i, node)| {
            let is_current = route.current_node.as_ref() == Some(node);
            let is_selected = i == selected_index;
            let is_testing = state.is_testing(node);
            let cached_delay = state.get_delay(node);
            let is_favorite = config.is_favorite(node);

            let (prefix, style) = if is_selected && is_current {
                (
                    "▶ ✓ ",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )
            } else if is_selected {
                (
                    "▶   ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
            } else if is_current {
                ("  ✓ ", Style::default().fg(Color::Green))
            } else {
                ("    ", Style::default().fg(Color::White))
            };

            let mut spans = vec![Span::styled(prefix, style)];

            // Add favorite indicator
            if is_favorite {
                spans.push(Span::styled("★ ", Style::default().fg(Color::Yellow)));
            }

            spans.push(Span::styled(node.clone(), style));

            // Show delay info if available
            if is_testing {
                spans.push(Span::styled(
                    " [Testing...]",
                    Style::default().fg(Color::Yellow),
                ));
            } else if let Some(delay_result) = cached_delay {
                let delay = delay_result.delay;
                let delay_style = if delay < 200 {
                    Style::default().fg(Color::Green)
                } else if delay < 500 {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Red)
                };

                let delay_text = if delay < 200 {
                    format!(" [{}ms ⚡Fast]", delay)
                } else if delay < 500 {
                    format!(" [{}ms Good]", delay)
                } else {
                    format!(" [{}ms Slow]", delay)
                };

                spans.push(Span::styled(delay_text, delay_style));
            }

            let content = Line::from(spans);
            ListItem::new(content)
        })
        .collect();

    let title_text = if state.preset.show_speed_test() {
        format!(
            "{} - Nodes ({}/{}) - Press 't' to test",
            route.display_name(),
            selected_index + 1,
            nodes.len()
        )
    } else {
        format!(
            "{} - Nodes ({}/{})",
            route.display_name(),
            selected_index + 1,
            nodes.len()
        )
    };

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(title_text));

    f.render_widget(list, area);
}

fn render_help(f: &mut Frame, area: Rect, _mode: Mode, preset: &Preset, expanded: bool) {
    let mut help_spans = vec![];

    if expanded {
        // Node selection mode help
        help_spans.extend(vec![
            Span::styled("↑↓", Style::default().fg(Color::Yellow)),
            Span::raw(" Navigate  "),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::raw(" Switch  "),
            Span::styled("*", Style::default().fg(Color::Yellow)),
            Span::raw(" Favorite  "),
        ]);

        // Show speed test only if preset allows
        if preset.show_speed_test() {
            help_spans.extend(vec![
                Span::styled("t", Style::default().fg(Color::Yellow)),
                Span::raw(" Test All  "),
            ]);
        }

        help_spans.extend(vec![
            Span::styled("Esc/q/←", Style::default().fg(Color::Yellow)),
            Span::raw(" Back  "),
            Span::styled("h", Style::default().fg(Color::Yellow)),
            Span::raw(" Home"),
        ]);
    } else {
        // Route list mode help
        help_spans.extend(vec![
            Span::styled("↑↓", Style::default().fg(Color::Yellow)),
            Span::raw(" Navigate  "),
            Span::styled("Enter/→", Style::default().fg(Color::Yellow)),
            Span::raw(" View Nodes  "),
        ]);

        // Show speed test only if preset allows
        if preset.show_speed_test() {
            help_spans.extend(vec![
                Span::styled("t", Style::default().fg(Color::Yellow)),
                Span::raw(" Test All  "),
            ]);
        }

        help_spans.extend(vec![
            Span::styled("h", Style::default().fg(Color::Yellow)),
            Span::raw(" Home  "),
            Span::styled("q/Esc", Style::default().fg(Color::Yellow)),
            Span::raw(" Back"),
        ]);
    }

    let help = Paragraph::new(Line::from(help_spans))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));

    f.render_widget(help, area);
}
