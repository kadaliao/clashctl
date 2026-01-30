use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::AppState;
use crate::clash::LogEntry;

/// Log level filter
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogLevel {
    All,
    Info,
    Warning,
    Error,
}

impl LogLevel {
    pub fn as_str(&self) -> &str {
        match self {
            LogLevel::All => "ALL",
            LogLevel::Info => "INFO",
            LogLevel::Warning => "WARNING",
            LogLevel::Error => "ERROR",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            LogLevel::All => LogLevel::Info,
            LogLevel::Info => LogLevel::Warning,
            LogLevel::Warning => LogLevel::Error,
            LogLevel::Error => LogLevel::All,
        }
    }

    pub fn color(&self) -> Color {
        match self {
            LogLevel::All => Color::Gray,
            LogLevel::Info => Color::Cyan,
            LogLevel::Warning => Color::Yellow,
            LogLevel::Error => Color::Red,
        }
    }
}

pub fn render(
    f: &mut Frame,
    area: Rect,
    _state: &AppState,
    logs: &[LogEntry],
    level_filter: LogLevel,
    search_query: &str,
    scroll_offset: usize,
    stream_connected: bool,
    stream_status: Option<&str>,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Filter bar
            Constraint::Min(0),    // Logs list
            Constraint::Length(3), // Help
        ])
        .split(area);

    render_filter_bar(
        f,
        chunks[0],
        level_filter,
        search_query,
        stream_connected,
        stream_status,
    );
    render_logs_list(
        f,
        chunks[1],
        logs,
        level_filter,
        search_query,
        scroll_offset,
    );
    render_help(f, chunks[2]);
}

fn render_filter_bar(
    f: &mut Frame,
    area: Rect,
    level_filter: LogLevel,
    search_query: &str,
    stream_connected: bool,
    stream_status: Option<&str>,
) {
    let is_connecting = matches!(stream_status, Some("connecting") | Some("reconnecting"));
    let status_label = if stream_connected {
        "Live"
    } else if is_connecting {
        "Connecting"
    } else {
        "Disconnected"
    };
    let status_color = if stream_connected {
        Color::Green
    } else if is_connecting {
        Color::Yellow
    } else {
        Color::Red
    };
    let status_detail = match stream_status {
        Some(detail) if !detail.is_empty() && !stream_connected && !is_connecting => Some(detail),
        _ => None,
    };

    let mut spans = Vec::new();
    spans.push(Span::raw("Filter: "));
    spans.push(Span::styled(
        level_filter.as_str(),
        Style::default().fg(level_filter.color()).add_modifier(Modifier::BOLD),
    ));
    spans.push(Span::raw(" | "));
    spans.push(Span::styled("● ", Style::default().fg(status_color)));
    spans.push(Span::styled(
        status_label,
        Style::default().fg(status_color).add_modifier(Modifier::BOLD),
    ));
    if let Some(detail) = status_detail {
        spans.push(Span::raw(" ("));
        spans.push(Span::styled(detail, Style::default().fg(Color::DarkGray)));
        spans.push(Span::raw(")"));
    }

    if search_query.is_empty() {
        spans.push(Span::raw(" | Press 'f' to change filter, '/' to search"));
    } else {
        spans.push(Span::raw(" | Search: \""));
        spans.push(Span::styled(search_query, Style::default().fg(Color::Yellow)));
        spans.push(Span::raw("\" | Press ESC to clear"));
    }

    let filter = Paragraph::new(Line::from(spans))
        .alignment(Alignment::Left)
        .block(Block::default().borders(Borders::ALL).title("Logs Filter"));

    f.render_widget(filter, area);
}

fn render_logs_list(
    f: &mut Frame,
    area: Rect,
    logs: &[LogEntry],
    level_filter: LogLevel,
    search_query: &str,
    scroll_offset: usize,
) {
    // Filter logs by level and search query
    let filtered_logs: Vec<&LogEntry> = logs
        .iter()
        .filter(|log| {
            // Filter by level
            let level_match = match level_filter {
                LogLevel::All => true,
                LogLevel::Info => log.level.to_uppercase().contains("INFO"),
                LogLevel::Warning => {
                    log.level.to_uppercase().contains("WARNING")
                        || log.level.to_uppercase().contains("WARN")
                }
                LogLevel::Error => log.level.to_uppercase().contains("ERROR"),
            };

            // Filter by search query
            let search_match = if search_query.is_empty() {
                true
            } else {
                log.message
                    .to_lowercase()
                    .contains(&search_query.to_lowercase())
                    || log
                        .level
                        .to_lowercase()
                        .contains(&search_query.to_lowercase())
            };

            level_match && search_match
        })
        .collect();

    let visible_count = (area.height as usize).saturating_sub(2); // Account for borders
    let display_logs = filtered_logs
        .iter()
        .skip(scroll_offset)
        .take(visible_count)
        .collect::<Vec<_>>();

    let items: Vec<ListItem> = display_logs
        .iter()
        .map(|log| {
            let level_color = if log.level.to_uppercase().contains("ERROR") {
                Color::Red
            } else if log.level.to_uppercase().contains("WARN") {
                Color::Yellow
            } else if log.level.to_uppercase().contains("INFO") {
                Color::Cyan
            } else {
                Color::Gray
            };

            let line = Line::from(vec![
                Span::styled(
                    format!("[{}] ", log.timestamp),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!("[{}] ", log.level),
                    Style::default()
                        .fg(level_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(&log.message),
            ]);

            ListItem::new(line)
        })
        .collect();

    let title = if filtered_logs.is_empty() {
        "Logs (No logs available)".to_string()
    } else {
        format!(
            "Logs ({} entries, showing {}-{})",
            filtered_logs.len(),
            scroll_offset + 1,
            (scroll_offset + visible_count).min(filtered_logs.len())
        )
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .style(Style::default().fg(Color::White));

    f.render_widget(list, area);
}

fn render_help(f: &mut Frame, area: Rect) {
    let help = Paragraph::new(Line::from(vec![
        Span::styled("↑↓", Style::default().fg(Color::Yellow)),
        Span::raw(" Scroll  "),
        Span::styled("f", Style::default().fg(Color::Yellow)),
        Span::raw(" Change Filter/Stream  "),
        Span::styled("/", Style::default().fg(Color::Yellow)),
        Span::raw(" Search  "),
        Span::styled("r", Style::default().fg(Color::Yellow)),
        Span::raw(" Reconnect  "),
        Span::styled("q/ESC", Style::default().fg(Color::Yellow)),
        Span::raw(" Back"),
    ]))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL));

    f.render_widget(help, area);
}
