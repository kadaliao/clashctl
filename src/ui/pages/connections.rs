use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::AppState;
use crate::clash::{Connection, ConnectionsResponse};

pub fn render(
    f: &mut Frame,
    area: Rect,
    _state: &AppState,
    connections: Option<&ConnectionsResponse>,
    selected_index: usize,
    scroll_offset: usize,
    search_query: &str,
    search_mode: bool,
) {
    let constraints = if search_mode {
        vec![
            Constraint::Length(3), // Title
            Constraint::Length(3), // Stats
            Constraint::Length(3), // Search input
            Constraint::Min(0),    // Connection list
            Constraint::Length(5), // Help
        ]
    } else {
        vec![
            Constraint::Length(3), // Title
            Constraint::Length(3), // Stats
            Constraint::Min(0),    // Connection list
            Constraint::Length(5), // Help
        ]
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let mut chunk_idx = 0;
    render_title(f, chunks[chunk_idx]);
    chunk_idx += 1;

    render_stats(f, chunks[chunk_idx], connections);
    chunk_idx += 1;

    if search_mode {
        render_search_input(f, chunks[chunk_idx], search_query);
        chunk_idx += 1;
    }

    render_connections(
        f,
        chunks[chunk_idx],
        connections,
        selected_index,
        scroll_offset,
        search_query,
    );
    chunk_idx += 1;

    render_help(f, chunks[chunk_idx], search_mode);
}

fn render_title(f: &mut Frame, area: Rect) {
    let title = Paragraph::new("Active Connections")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, area);
}

fn render_stats(f: &mut Frame, area: Rect, connections: Option<&ConnectionsResponse>) {
    let (count, upload, download) = if let Some(conn) = connections {
        (
            conn.connections.len(),
            format_bytes(conn.upload_total),
            format_bytes(conn.download_total),
        )
    } else {
        (0, "0 B".to_string(), "0 B".to_string())
    };

    let stats = Line::from(vec![
        Span::raw("Total: "),
        Span::styled(
            format!("{}", count),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  |  Upload: "),
        Span::styled(upload, Style::default().fg(Color::Yellow)),
        Span::raw("  |  Download: "),
        Span::styled(download, Style::default().fg(Color::Cyan)),
    ]);

    let widget = Paragraph::new(stats)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("Statistics"));

    f.render_widget(widget, area);
}

fn render_search_input(f: &mut Frame, area: Rect, search_query: &str) {
    let search_text = if search_query.is_empty() {
        Line::from(vec![
            Span::styled("Search: ", Style::default().fg(Color::Cyan)),
            Span::styled("_", Style::default().fg(Color::Gray)),
        ])
    } else {
        Line::from(vec![
            Span::styled("Search: ", Style::default().fg(Color::Cyan)),
            Span::raw(search_query),
            Span::styled("_", Style::default().fg(Color::Yellow)),
        ])
    };

    let search_widget = Paragraph::new(search_text)
        .alignment(Alignment::Left)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Search (Host/IP/Chain)"),
        );

    f.render_widget(search_widget, area);
}

fn render_connections(
    f: &mut Frame,
    area: Rect,
    connections: Option<&ConnectionsResponse>,
    selected_index: usize,
    scroll_offset: usize,
    search_query: &str,
) {
    let items: Vec<ListItem> = if let Some(conn) = connections {
        if conn.connections.is_empty() {
            vec![ListItem::new(Line::from(vec![Span::styled(
                "No active connections",
                Style::default().fg(Color::Gray),
            )]))]
        } else {
            // Filter connections based on search query
            let filtered: Vec<(usize, &Connection)> = if search_query.is_empty() {
                conn.connections.iter().enumerate().collect()
            } else {
                conn.connections
                    .iter()
                    .enumerate()
                    .filter(|(_, connection)| {
                        let query_lower = search_query.to_lowercase();

                        // Search in destination host
                        if let Some(host) = &connection.metadata.host {
                            if host.to_lowercase().contains(&query_lower) {
                                return true;
                            }
                        }

                        // Search in destination IP
                        if connection
                            .metadata
                            .destination_ip
                            .to_lowercase()
                            .contains(&query_lower)
                        {
                            return true;
                        }

                        // Search in source IP
                        if connection
                            .metadata
                            .source_ip
                            .to_lowercase()
                            .contains(&query_lower)
                        {
                            return true;
                        }

                        // Search in chains
                        for chain in &connection.chains {
                            if chain.to_lowercase().contains(&query_lower) {
                                return true;
                            }
                        }

                        false
                    })
                    .collect()
            };

            if filtered.is_empty() {
                vec![ListItem::new(Line::from(vec![Span::styled(
                    format!("No connections matching '{}'", search_query),
                    Style::default().fg(Color::Yellow),
                )]))]
            } else {
                filtered
                    .iter()
                    .skip(scroll_offset)
                    .map(|(idx, connection)| {
                        render_connection_item(connection, *idx == selected_index)
                    })
                    .collect()
            }
        }
    } else {
        vec![ListItem::new(Line::from(vec![Span::styled(
            "Loading connections...",
            Style::default().fg(Color::Yellow),
        )]))]
    };

    let title = if search_query.is_empty() {
        format!("Connections (offset: {})", scroll_offset)
    } else {
        format!(
            "Connections (filtered: '{}', offset: {})",
            search_query, scroll_offset
        )
    };

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(title));

    f.render_widget(list, area);
}

fn render_connection_item(connection: &Connection, is_selected: bool) -> ListItem<'_> {
    let style = if is_selected {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    let prefix = if is_selected { "► " } else { "  " };

    // Format connection details
    let network = connection.metadata.network.to_uppercase();
    let source = format!(
        "{}:{}",
        connection.metadata.source_ip, connection.metadata.source_port
    );
    let dest = if let Some(host) = &connection.metadata.host {
        format!("{}:{}", host, connection.metadata.destination_port)
    } else {
        format!(
            "{}:{}",
            connection.metadata.destination_ip, connection.metadata.destination_port
        )
    };

    let chain = if !connection.chains.is_empty() {
        connection.chains.join(" → ")
    } else {
        "DIRECT".to_string()
    };

    let upload_str = format_bytes(connection.upload);
    let download_str = format_bytes(connection.download);

    let line1 = Line::from(vec![
        Span::styled(prefix, style),
        Span::styled(format!("[{}] ", network), Style::default().fg(Color::Cyan)),
        Span::styled(source, Style::default().fg(Color::Green)),
        Span::raw(" → "),
        Span::styled(dest, Style::default().fg(Color::Yellow)),
    ]);

    let line2 = Line::from(vec![
        Span::raw("    "),
        Span::styled("Chain: ", Style::default().fg(Color::Gray)),
        Span::styled(chain, Style::default().fg(Color::Magenta)),
        Span::raw("  |  "),
        Span::styled("↑ ", Style::default().fg(Color::Green)),
        Span::raw(upload_str),
        Span::raw("  "),
        Span::styled("↓ ", Style::default().fg(Color::Cyan)),
        Span::raw(download_str),
    ]);

    ListItem::new(vec![line1, line2])
}

fn render_help(f: &mut Frame, area: Rect, search_mode: bool) {
    let help_spans = if search_mode {
        vec![
            Span::styled("Esc", Style::default().fg(Color::Yellow)),
            Span::raw(" Exit Search  "),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::raw(" Apply Filter"),
        ]
    } else {
        vec![
            Span::styled("/", Style::default().fg(Color::Yellow)),
            Span::raw(" Search  "),
            Span::styled("↑↓", Style::default().fg(Color::Yellow)),
            Span::raw(" Navigate  "),
            Span::styled("d", Style::default().fg(Color::Yellow)),
            Span::raw(" Close Connection  "),
            Span::styled("a", Style::default().fg(Color::Yellow)),
            Span::raw(" Close All  "),
            Span::styled("r", Style::default().fg(Color::Yellow)),
            Span::raw(" Refresh  "),
            Span::styled("h", Style::default().fg(Color::Yellow)),
            Span::raw(" Home  "),
            Span::styled("q", Style::default().fg(Color::Yellow)),
            Span::raw(" Back"),
        ]
    };

    let help = Paragraph::new(Line::from(help_spans))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));

    f.render_widget(help, area);
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    if unit_idx == 0 {
        format!("{} {}", size as u64, UNITS[unit_idx])
    } else {
        format!("{:.2} {}", size, UNITS[unit_idx])
    }
}
