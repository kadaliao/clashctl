use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::AppState;

pub fn render(f: &mut Frame, area: Rect, state: &AppState, providers: &[(String, String, Option<String>, usize, Option<String>)], selected_index: usize) {
    let constraints = if state.status_message.is_some() {
        vec![
            Constraint::Length(3),  // Title
            Constraint::Length(3),  // Status message
            Constraint::Min(0),     // Content
            Constraint::Length(5),  // Help
        ]
    } else {
        vec![
            Constraint::Length(3),  // Title
            Constraint::Min(0),     // Content
            Constraint::Length(5),  // Help
        ]
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let mut chunk_idx = 0;
    render_title(f, chunks[chunk_idx]);
    chunk_idx += 1;

    if let Some(msg) = &state.status_message {
        render_status(f, chunks[chunk_idx], msg);
        chunk_idx += 1;
    }

    render_providers(f, chunks[chunk_idx], providers, selected_index);
    chunk_idx += 1;

    render_help(f, chunks[chunk_idx]);
}

fn render_title(f: &mut Frame, area: Rect) {
    let title_text = "Subscription Management (订阅管理)";
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

fn render_providers(f: &mut Frame, area: Rect, providers: &[(String, String, Option<String>, usize, Option<String>)], selected_index: usize) {
    if providers.is_empty() {
        let content = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("No Subscriptions Found", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from("No proxy subscriptions (订阅) are configured in your Clash configuration."),
            Line::from(""),
            Line::from(vec![
                Span::styled("What are subscriptions?", Style::default().fg(Color::Cyan)),
            ]),
            Line::from("  Subscriptions are remote URLs provided by airport services (机场)."),
            Line::from("  They automatically fetch and update proxy server lists."),
            Line::from(""),
            Line::from(vec![
                Span::styled("To add subscriptions:", Style::default().fg(Color::Green)),
            ]),
            Line::from("  1. Get subscription URL from your airport provider"),
            Line::from("  2. Edit your Clash config file (config.yaml)"),
            Line::from("  3. Add a 'proxy-providers' section with your subscription URLs"),
            Line::from("  4. Restart Clash"),
            Line::from("  5. Press 'r' here to refresh"),
            Line::from(""),
            Line::from(vec![
                Span::styled("Example:", Style::default().fg(Color::Magenta)),
            ]),
            Line::from("  proxy-providers:"),
            Line::from("    my-airport:"),
            Line::from("      type: http"),
            Line::from("      url: \"https://your-airport.com/sub?token=xxx\""),
            Line::from("      interval: 3600"),
        ];

        let paragraph = Paragraph::new(content)
            .alignment(Alignment::Left)
            .block(Block::default().borders(Borders::ALL).title("Subscriptions (订阅)"));

        f.render_widget(paragraph, area);
        return;
    }

    // Render provider list with selection
    let items: Vec<ListItem> = providers
        .iter()
        .enumerate()
        .map(|(idx, (name, ptype, url, proxy_count, updated_at))| {
            let updated_str = if let Some(time) = updated_at {
                format!("Updated: {}", time)
            } else {
                "Never updated".to_string()
            };

            let url_display = if let Some(u) = url {
                if u.len() > 80 {
                    format!("{}...", &u[..80])
                } else {
                    u.clone()
                }
            } else {
                "No URL".to_string()
            };

            let is_selected = idx == selected_index;

            let line1 = Line::from(vec![
                Span::styled(
                    if is_selected { "▶ " } else { "  " },
                    Style::default().fg(if is_selected { Color::Yellow } else { Color::White }),
                ),
                Span::styled(
                    name,
                    Style::default()
                        .fg(if is_selected { Color::Cyan } else { Color::White })
                        .add_modifier(if is_selected { Modifier::BOLD } else { Modifier::empty() }),
                ),
                Span::raw("  "),
                Span::styled(
                    format!("[{}]", ptype),
                    Style::default().fg(Color::Green),
                ),
                Span::raw("  "),
                Span::styled(
                    format!("({} nodes)", proxy_count),
                    Style::default().fg(if is_selected { Color::Yellow } else { Color::DarkGray }),
                ),
            ]);

            let line2 = Line::from(vec![
                Span::raw(if is_selected { "   " } else { "     " }),
                Span::styled("URL: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    url_display,
                    Style::default().fg(if is_selected { Color::Cyan } else { Color::DarkGray }),
                ),
            ]);

            let line3 = Line::from(vec![
                Span::raw(if is_selected { "   " } else { "     " }),
                Span::styled(updated_str, Style::default().fg(Color::DarkGray)),
            ]);

            ListItem::new(vec![line1, line2, line3])
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Your Subscriptions (订阅) - {} total", providers.len())),
        );

    f.render_widget(list, area);
}

fn render_help(f: &mut Frame, area: Rect) {
    let help_spans = vec![
        Span::styled("↑↓", Style::default().fg(Color::Yellow)),
        Span::raw(" Select  "),
        Span::styled("Enter", Style::default().fg(Color::Yellow)),
        Span::raw(" Update Selected  "),
        Span::styled("u", Style::default().fg(Color::Yellow)),
        Span::raw(" Update All  "),
        Span::styled("r", Style::default().fg(Color::Yellow)),
        Span::raw(" Refresh  "),
        Span::styled("q", Style::default().fg(Color::Yellow)),
        Span::raw(" Back"),
    ];

    let help = Paragraph::new(Line::from(help_spans))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));

    f.render_widget(help, area);
}
