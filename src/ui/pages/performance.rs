use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::AppState;

/// Format bytes to human readable format
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Format rate to human readable format (bytes per second)
fn format_rate(bytes_per_sec: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;

    if bytes_per_sec >= MB {
        format!("{:.2} MB/s", bytes_per_sec as f64 / MB as f64)
    } else if bytes_per_sec >= KB {
        format!("{:.2} KB/s", bytes_per_sec as f64 / KB as f64)
    } else {
        format!("{} B/s", bytes_per_sec)
    }
}

pub fn render(
    f: &mut Frame,
    area: Rect,
    _state: &AppState,
    upload_total: u64,
    download_total: u64,
    upload_rate: u64,
    download_rate: u64,
    connection_count: usize,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Title
            Constraint::Length(7),  // Traffic stats
            Constraint::Length(10), // Rate graph
            Constraint::Min(0),     // Connection info
            Constraint::Length(3),  // Help
        ])
        .split(area);

    // Title
    let title = Paragraph::new("Performance Monitor")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    // Traffic stats
    render_traffic_stats(
        f,
        chunks[1],
        upload_total,
        download_total,
        connection_count,
    );

    // Rate graph
    render_rate_graph(f, chunks[2], upload_rate, download_rate);

    // Connection info
    render_connection_info(f, chunks[3], connection_count);

    // Help
    let help = Paragraph::new(Line::from(vec![
        Span::styled("r", Style::default().fg(Color::Yellow)),
        Span::raw(" Refresh  "),
        Span::styled("c", Style::default().fg(Color::Yellow)),
        Span::raw(" Connections  "),
        Span::styled("q/ESC", Style::default().fg(Color::Yellow)),
        Span::raw(" Back  "),
        Span::raw("Auto-refresh: Every 5s"),
    ]))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(help, chunks[4]);
}

fn render_traffic_stats(
    f: &mut Frame,
    area: Rect,
    upload_total: u64,
    download_total: u64,
    connection_count: usize,
) {
    let stats = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Total Upload:   ", Style::default().fg(Color::Gray)),
            Span::styled(
                format_bytes(upload_total),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Total Download: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format_bytes(download_total),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Connections:    ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{}", connection_count),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Traffic Statistics"),
    );
    f.render_widget(stats, area);
}

fn render_rate_graph(f: &mut Frame, area: Rect, upload_rate: u64, download_rate: u64) {
    // Simple text-based visualization
    let max_rate = upload_rate.max(download_rate);
    let max_display = if max_rate == 0 { 100 } else { max_rate };

    let upload_bars = ((upload_rate as f64 / max_display as f64) * 40.0) as usize;
    let download_bars = ((download_rate as f64 / max_display as f64) * 40.0) as usize;

    let upload_bar = "█".repeat(upload_bars);
    let download_bar = "█".repeat(download_bars);

    let graph = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Upload:   ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{:<40}", upload_bar),
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(
                format!(" {}", format_rate(upload_rate)),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Download: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{:<40}", download_bar),
                Style::default().fg(Color::Green),
            ),
            Span::styled(
                format!(" {}", format_rate(download_rate)),
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Scale: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("0 → {}", format_rate(max_display)),
                Style::default().fg(Color::DarkGray),
            ),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Current Rate"),
    );
    f.render_widget(graph, area);
}

fn render_connection_info(f: &mut Frame, area: Rect, connection_count: usize) {
    let status_text = if connection_count == 0 {
        "No active connections"
    } else if connection_count < 10 {
        "Low activity"
    } else if connection_count < 50 {
        "Normal activity"
    } else if connection_count < 100 {
        "High activity"
    } else {
        "Very high activity"
    };

    let status_color = if connection_count == 0 {
        Color::Gray
    } else if connection_count < 10 {
        Color::Green
    } else if connection_count < 50 {
        Color::Cyan
    } else if connection_count < 100 {
        Color::Yellow
    } else {
        Color::Red
    };

    let info = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Connection Status: ", Style::default().fg(Color::Gray)),
            Span::styled(
                status_text,
                Style::default()
                    .fg(status_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "Go to Connections page (press 'c' on Home) for details",
                Style::default().fg(Color::DarkGray),
            ),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Connection Info"),
    )
    .alignment(Alignment::Left);
    f.render_widget(info, area);
}
