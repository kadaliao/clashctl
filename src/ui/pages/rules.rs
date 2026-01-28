use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::AppState;
use crate::config::AppConfig;
use crate::clash::Rule;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RuleEditMode {
    None,
    AddWhitelist,
    AddBlacklist,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RuleListFocus {
    Whitelist,
    Blacklist,
}

pub fn render(
    f: &mut Frame,
    area: Rect,
    state: &AppState,
    scroll_offset: usize,
    search_query: &str,
    search_mode: bool,
    edit_mode: RuleEditMode,
    _edit_input: &str,
    _config: &AppConfig,
    _selected_index: usize,
    rules: &[Rule],
    _list_focus: RuleListFocus,
) {
    let mut constraints = vec![Constraint::Length(3)]; // Title

    if state.status_message.is_some() {
        constraints.push(Constraint::Length(3)); // Status message
    }

    if search_mode {
        constraints.push(Constraint::Length(3)); // Search input
    }

    constraints.push(Constraint::Min(0)); // Content
    constraints.push(Constraint::Length(5)); // Help

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

    if search_mode {
        render_search_input(f, chunks[chunk_idx], search_query);
        chunk_idx += 1;
    }

    // Always show all rules (expert mode)
    render_all_rules(f, chunks[chunk_idx], state, scroll_offset, search_query, rules);
    chunk_idx += 1;

    render_help(f, chunks[chunk_idx], search_mode, edit_mode);
}

fn render_title(f: &mut Frame, area: Rect) {
    let title_text = "Rules Management (规则管理)";
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

fn render_all_rules(f: &mut Frame, area: Rect, _state: &AppState, scroll_offset: usize, search_query: &str, rules: &[Rule]) {
    let available_width = area.width.saturating_sub(4) as usize; // Subtract borders and padding
    if rules.is_empty() {
        let content = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("No Rules Loaded", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from("Failed to load rules from Clash API."),
            Line::from(""),
            Line::from(vec![
                Span::styled("Troubleshooting:", Style::default().fg(Color::Cyan)),
            ]),
            Line::from("  • Make sure Clash is running"),
            Line::from("  • Check the API connection settings"),
            Line::from("  • Try refreshing with 'r' key"),
        ];

        let paragraph = Paragraph::new(content)
            .alignment(Alignment::Left)
            .block(Block::default().borders(Borders::ALL).title("Rules"));

        f.render_widget(paragraph, area);
        return;
    }

    // Filter rules based on search query
    let filtered_rules: Vec<&Rule> = if search_query.is_empty() {
        rules.iter().collect()
    } else {
        let query_lower = search_query.to_lowercase();
        rules
            .iter()
            .filter(|rule| {
                rule.rule_type.to_lowercase().contains(&query_lower)
                    || rule.payload.to_lowercase().contains(&query_lower)
                    || rule.proxy.to_lowercase().contains(&query_lower)
            })
            .collect()
    };

    if filtered_rules.is_empty() {
        let message = format!("No rules matching '{}'", search_query);
        let content = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(message, Style::default().fg(Color::Yellow)),
            ]),
        ];
        let paragraph = Paragraph::new(content)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Rules"));
        f.render_widget(paragraph, area);
        return;
    }

    // Render rule list
    let items: Vec<ListItem> = filtered_rules
        .iter()
        .skip(scroll_offset)
        .take(area.height as usize - 2)
        .map(|rule| {
            let rule_type_color = match rule.rule_type.as_str() {
                "DOMAIN" => Color::Cyan,
                "DOMAIN-SUFFIX" => Color::Blue,
                "DOMAIN-KEYWORD" => Color::Magenta,
                "IP-CIDR" => Color::Green,
                "GEOIP" => Color::Yellow,
                "MATCH" => Color::Red,
                _ => Color::White,
            };

            // Smart column width allocation based on available space
            // Priority: ensure proxy is always visible
            let rule_type_width = 13; // Fixed width for rule type
            let arrow_width = 3; // " → "
            let spacing_width = 2; // Two single spaces
            let min_proxy_width = 15; // Minimum width to show proxy

            // Calculate available width for payload
            let reserved_width = rule_type_width + arrow_width + spacing_width + min_proxy_width;
            let payload_max_width = if available_width > reserved_width {
                (available_width - reserved_width).min(40)
            } else {
                20 // Fallback minimum
            };

            // Format rule type (fixed width with padding)
            let rule_type_str = if rule.rule_type.len() > rule_type_width {
                format!("{:.10}...", &rule.rule_type[..10])
            } else {
                format!("{:width$}", rule.rule_type, width = rule_type_width)
            };

            // Format payload (truncate if needed, no padding)
            let payload_str = if rule.payload.len() > payload_max_width {
                format!("{}...", &rule.payload[..payload_max_width.saturating_sub(3)])
            } else {
                rule.payload.clone()
            };

            // Format proxy (truncate if needed, no padding)
            let proxy_max_width = 25;
            let proxy_str = if rule.proxy.len() > proxy_max_width {
                format!("{}...", &rule.proxy[..proxy_max_width.saturating_sub(3)])
            } else {
                rule.proxy.clone()
            };

            let line = Line::from(vec![
                Span::styled(
                    rule_type_str,
                    Style::default().fg(rule_type_color),
                ),
                Span::raw(" "),
                Span::styled(
                    payload_str,
                    Style::default().fg(Color::White),
                ),
                Span::raw(" → "),
                Span::styled(
                    proxy_str,
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                ),
            ]);
            ListItem::new(line)
        })
        .collect();

    let title = if search_query.is_empty() {
        format!("All Rules - {} total (offset: {})", filtered_rules.len(), scroll_offset)
    } else {
        format!("Filtered Rules - {} matches (offset: {})", filtered_rules.len(), scroll_offset)
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title));

    f.render_widget(list, area);
}

fn render_help(f: &mut Frame, area: Rect, search_mode: bool, edit_mode: RuleEditMode) {
    let help_spans = if edit_mode != RuleEditMode::None {
        vec![
            Span::styled("Esc", Style::default().fg(Color::Yellow)),
            Span::raw(" Cancel  "),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::raw(" Done"),
        ]
    } else if search_mode {
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
            Span::raw(" Scroll  "),
            Span::styled("r", Style::default().fg(Color::Yellow)),
            Span::raw(" Refresh  "),
            Span::styled("q", Style::default().fg(Color::Yellow)),
            Span::raw(" Back"),
        ]
    };

    let help = Paragraph::new(Line::from(help_spans))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));

    f.render_widget(help, area);
}

fn render_search_input(f: &mut Frame, area: Rect, search_query: &str) {
    let search_text = if search_query.is_empty() {
        Line::from(vec![
            Span::styled("Filter: ", Style::default().fg(Color::Cyan)),
            Span::styled("_", Style::default().fg(Color::Gray)),
        ])
    } else {
        Line::from(vec![
            Span::styled("Filter: ", Style::default().fg(Color::Cyan)),
            Span::raw(search_query),
            Span::styled("_", Style::default().fg(Color::Yellow)),
        ])
    };

    let search_widget = Paragraph::new(search_text)
        .alignment(Alignment::Left)
        .block(Block::default().borders(Borders::ALL).title("Search (Type/Payload/Proxy)"));

    f.render_widget(search_widget, area);
}
