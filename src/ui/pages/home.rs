use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::AppState;

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let constraints = if state.status_message.is_some() {
        vec![
            Constraint::Length(5), // Status box
            Constraint::Length(3), // Status message
            Constraint::Min(0),    // Quick actions
            Constraint::Length(3), // Help
        ]
    } else {
        vec![
            Constraint::Length(5), // Status box
            Constraint::Min(0),    // Quick actions
            Constraint::Length(3), // Help
        ]
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let mut chunk_idx = 0;
    render_status(f, chunks[chunk_idx], state);
    chunk_idx += 1;

    if let Some(msg) = &state.status_message {
        render_status_message(f, chunks[chunk_idx], msg);
        chunk_idx += 1;
    }

    render_quick_actions(f, chunks[chunk_idx]);
    chunk_idx += 1;

    render_help(f, chunks[chunk_idx]);
}

fn render_status(f: &mut Frame, area: Rect, state: &AppState) {
    let clash = &state.clash_state;

    let mode_str = format!("{:?} Mode", clash.mode);
    let route_str = if let Some(proxy) = &clash.current_proxy {
        format!("Route: {}", proxy)
    } else {
        "Route: None".to_string()
    };

    let health = clash.get_health_status();
    let health_line = Line::from(vec![
        Span::raw("Health: "),
        Span::styled(
            health.as_str(),
            Style::default()
                .fg(health.color())
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let mut lines = vec![
        Line::from(Span::styled(
            mode_str,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(route_str),
        health_line,
    ];

    // Show current node speed test result if available
    if let Some(current_node) = state.get_current_node() {
        if state.is_testing(&current_node) {
            lines.push(Line::from(vec![
                Span::raw("Speed: "),
                Span::styled("Testing...", Style::default().fg(Color::Yellow)),
            ]));
        } else if let Some(delay_result) = state.get_delay(&current_node) {
            let delay = delay_result.delay;
            let (delay_text, delay_color) = if delay < 200 {
                (format!("{}ms ⚡Fast", delay), Color::Green)
            } else if delay < 500 {
                (format!("{}ms Good", delay), Color::Yellow)
            } else {
                (format!("{}ms Slow", delay), Color::Red)
            };
            lines.push(Line::from(vec![
                Span::raw("Speed: "),
                Span::styled(
                    delay_text,
                    Style::default()
                        .fg(delay_color)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
        }
    }

    // Error display with helpful hints
    if let Some(err) = &clash.error {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "⚠ Connection Error:",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )));

        // Parse error and provide helpful hints
        if err.contains("401") || err.contains("Unauthorized") {
            lines.push(Line::from(Span::styled(
                "  Authentication required",
                Style::default().fg(Color::Red),
            )));
            lines.push(Line::from(Span::styled(
                "  Try: cargo run -- --secret YOUR_SECRET",
                Style::default().fg(Color::Yellow),
            )));
        } else if err.contains("Connection refused") || err.contains("connect") {
            lines.push(Line::from(Span::styled(
                "  Cannot connect to Clash",
                Style::default().fg(Color::Red),
            )));
            lines.push(Line::from(Span::styled(
                "  Make sure Clash is running on port 9090",
                Style::default().fg(Color::Yellow),
            )));
        } else {
            // Generic error
            lines.push(Line::from(Span::styled(
                format!("  {}", err),
                Style::default().fg(Color::Red),
            )));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Press 'r' to retry",
            Style::default().fg(Color::Cyan),
        )));
    }

    let status = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Status"))
        .alignment(Alignment::Left);

    f.render_widget(status, area);
}

fn render_status_message(f: &mut Frame, area: Rect, message: &str) {
    let msg = Paragraph::new(message)
        .style(Style::default().fg(Color::Yellow))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(msg, area);
}

fn render_quick_actions(f: &mut Frame, area: Rect) {
    let actions = Paragraph::new(vec![
        Line::from(""),
        Line::from("Quick Actions:"),
        Line::from(""),
        Line::from(vec![
            Span::styled("  [m]", Style::default().fg(Color::Yellow)),
            Span::raw(" Switch Scene (Rule/Global/Direct)"),
        ]),
        Line::from(vec![
            Span::styled("  [g]", Style::default().fg(Color::Yellow)),
            Span::raw(" Go to Routes (Node Management)"),
        ]),
        Line::from(vec![
            Span::styled("  [l]", Style::default().fg(Color::Yellow)),
            Span::raw(" Go to Rules"),
        ]),
        Line::from(vec![
            Span::styled("  [c]", Style::default().fg(Color::Yellow)),
            Span::raw(" Go to Connections"),
        ]),
        Line::from(vec![
            Span::styled("  [p]", Style::default().fg(Color::Yellow)),
            Span::raw(" Go to Performance"),
        ]),
        Line::from(vec![
            Span::styled("  [o]", Style::default().fg(Color::Yellow)),
            Span::raw(" Go to Logs"),
        ]),
        Line::from(vec![
            Span::styled("  [u]", Style::default().fg(Color::Yellow)),
            Span::raw(" Go to Update"),
        ]),
        Line::from(vec![
            Span::styled("  [s]", Style::default().fg(Color::Yellow)),
            Span::raw(" Go to Settings"),
        ]),
        Line::from(vec![
            Span::styled("  [r]", Style::default().fg(Color::Yellow)),
            Span::raw(" Refresh Status"),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Quick Actions"),
    )
    .alignment(Alignment::Left);

    f.render_widget(actions, area);
}

fn render_help(f: &mut Frame, area: Rect) {
    let help = Paragraph::new(Line::from(vec![
        Span::styled("q", Style::default().fg(Color::Yellow)),
        Span::raw(" Quit  "),
        Span::styled("?", Style::default().fg(Color::Yellow)),
        Span::raw(" Help"),
    ]))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL));

    f.render_widget(help, area);
}
