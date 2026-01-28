use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::AppState;
use crate::config::AppConfig;

pub enum SettingsAction {
    None,
    ExportPrompt,
    ImportPrompt,
    ExportSuccess(String),
    ImportSuccess,
    Error(String),
}

pub fn render(
    f: &mut Frame,
    area: Rect,
    _state: &AppState,
    config: &AppConfig,
    action: &SettingsAction,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(0),    // Settings options
            Constraint::Length(5), // Help
        ])
        .split(area);

    render_title(f, chunks[0]);
    render_settings(f, chunks[1], config, action);
    render_help(f, chunks[2], action);
}

fn render_title(f: &mut Frame, area: Rect) {
    let title = Paragraph::new("Settings & Configuration")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, area);
}

fn render_settings(f: &mut Frame, area: Rect, config: &AppConfig, action: &SettingsAction) {
    let mut lines = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "Configuration Management",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  [e]", Style::default().fg(Color::Green)),
            Span::raw(" Export Configuration to File"),
        ]),
        Line::from(vec![
            Span::styled("  [i]", Style::default().fg(Color::Green)),
            Span::raw(" Import Configuration from File"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Current Configuration:",
            Style::default().fg(Color::Cyan),
        )]),
        Line::from(vec![
            Span::raw("  API URL: "),
            Span::styled(&config.api_url, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::raw("  Secret: "),
            Span::styled(
                if config.secret.is_some() {
                    "✓ Configured"
                } else {
                    "Not set"
                },
                if config.secret.is_some() {
                    Color::Green
                } else {
                    Color::Gray
                },
            ),
        ]),
        Line::from(vec![
            Span::raw("  Preset: "),
            Span::styled(&config.current_preset, Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::raw("  Whitelist Rules: "),
            Span::styled(
                config.whitelist.len().to_string(),
                Style::default().fg(Color::Green),
            ),
        ]),
        Line::from(vec![
            Span::raw("  Blacklist Rules: "),
            Span::styled(
                config.blacklist.len().to_string(),
                Style::default().fg(Color::Red),
            ),
        ]),
        Line::from(vec![
            Span::raw("  Favorite Nodes: "),
            Span::styled(
                config.favorite_nodes.len().to_string(),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(""),
    ];

    // Show action-specific messages
    match action {
        SettingsAction::ExportPrompt => {
            lines.push(Line::from(vec![
                Span::styled("Export Path: ", Style::default().fg(Color::Yellow)),
                Span::raw("~/.config/clashctl/clashctl-export.yaml"),
            ]));
            lines.push(Line::from(vec![Span::styled(
                "Press 'y' to confirm export",
                Style::default().fg(Color::Green),
            )]));
        }
        SettingsAction::ImportPrompt => {
            lines.push(Line::from(vec![
                Span::styled("Import Path: ", Style::default().fg(Color::Yellow)),
                Span::raw("~/.config/clashctl/clashctl-import.yaml"),
            ]));
            lines.push(Line::from(vec![Span::styled(
                "Press 'y' to confirm import (will restart app)",
                Style::default().fg(Color::Red),
            )]));
            lines.push(Line::from(vec![Span::styled(
                "Warning: Current config will be replaced!",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )]));
        }
        SettingsAction::ExportSuccess(path) => {
            lines.push(Line::from(vec![
                Span::styled("✓ ", Style::default().fg(Color::Green)),
                Span::styled(
                    "Configuration exported successfully!",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::raw("  Location: "),
                Span::styled(path, Style::default().fg(Color::Cyan)),
            ]));
        }
        SettingsAction::ImportSuccess => {
            lines.push(Line::from(vec![
                Span::styled("✓ ", Style::default().fg(Color::Green)),
                Span::styled(
                    "Configuration imported successfully!",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
            lines.push(Line::from(vec![Span::raw(
                "  Please restart the application to apply changes",
            )]));
        }
        SettingsAction::Error(err) => {
            lines.push(Line::from(vec![
                Span::styled("✗ ", Style::default().fg(Color::Red)),
                Span::styled(
                    "Error:",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(err, Style::default().fg(Color::Red)),
            ]));
        }
        SettingsAction::None => {}
    }

    let settings = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Settings"))
        .alignment(Alignment::Left);

    f.render_widget(settings, area);
}

fn render_help(f: &mut Frame, area: Rect, action: &SettingsAction) {
    let help_spans = match action {
        SettingsAction::ExportPrompt | SettingsAction::ImportPrompt => vec![
            Span::styled("y", Style::default().fg(Color::Yellow)),
            Span::raw(" Confirm  "),
            Span::styled("n/Esc", Style::default().fg(Color::Yellow)),
            Span::raw(" Cancel"),
        ],
        _ => vec![
            Span::styled("e", Style::default().fg(Color::Yellow)),
            Span::raw(" Export  "),
            Span::styled("i", Style::default().fg(Color::Yellow)),
            Span::raw(" Import  "),
            Span::styled("h", Style::default().fg(Color::Yellow)),
            Span::raw(" Home  "),
            Span::styled("q", Style::default().fg(Color::Yellow)),
            Span::raw(" Back"),
        ],
    };

    let help = Paragraph::new(Line::from(help_spans))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));

    f.render_widget(help, area);
}
