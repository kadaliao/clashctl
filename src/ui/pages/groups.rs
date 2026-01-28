use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::AppState;
use crate::config::AppConfig;

#[derive(Debug, Clone, PartialEq)]
pub enum GroupsAction {
    None,
    CreateGroup(String),  // Group name input
    SelectingNodes(String),  // Group name, selecting nodes to add
    ViewingGroup(String),  // Viewing nodes in a group
}

pub fn render(
    f: &mut Frame,
    area: Rect,
    state: &AppState,
    config: &AppConfig,
    action: &GroupsAction,
    input: &str,
    selected_index: usize,
) {
    match action {
        GroupsAction::None => render_group_list(f, area, state, config, selected_index),
        GroupsAction::CreateGroup(_) => render_create_group(f, area, state, input),
        GroupsAction::SelectingNodes(group_name) => {
            render_node_selection(f, area, state, config, group_name, selected_index)
        }
        GroupsAction::ViewingGroup(group_name) => {
            render_group_view(f, area, state, config, group_name, selected_index)
        }
    }
}

fn render_group_list(
    f: &mut Frame,
    area: Rect,
    _state: &AppState,
    config: &AppConfig,
    selected_index: usize,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(0),    // Group list
            Constraint::Length(3), // Help
        ])
        .split(area);

    // Title
    let title = Paragraph::new("Node Groups")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    // Group list
    let group_names = config.get_group_names();
    let items: Vec<ListItem> = group_names
        .iter()
        .enumerate()
        .map(|(idx, name)| {
            let nodes = config.get_group_nodes(name).map(|n| n.len()).unwrap_or(0);
            let line = if idx == selected_index {
                Line::from(vec![
                    Span::styled("▶ ", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        name,
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!(" ({} nodes)", nodes),
                        Style::default().fg(Color::Gray),
                    ),
                ])
            } else {
                Line::from(vec![
                    Span::raw("  "),
                    Span::raw(name),
                    Span::styled(
                        format!(" ({} nodes)", nodes),
                        Style::default().fg(Color::Gray),
                    ),
                ])
            };
            ListItem::new(line)
        })
        .collect();

    let list_title = if group_names.is_empty() {
        "Groups (No groups created)".to_string()
    } else {
        format!("Groups ({} total)", group_names.len())
    };

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(list_title));
    f.render_widget(list, chunks[1]);

    // Help
    let help = Paragraph::new(Line::from(vec![
        Span::styled("n", Style::default().fg(Color::Yellow)),
        Span::raw(" New Group  "),
        Span::styled("Enter", Style::default().fg(Color::Yellow)),
        Span::raw(" View/Edit  "),
        Span::styled("d", Style::default().fg(Color::Yellow)),
        Span::raw(" Delete  "),
        Span::styled("q/ESC", Style::default().fg(Color::Yellow)),
        Span::raw(" Back"),
    ]))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(help, chunks[2]);
}

fn render_create_group(f: &mut Frame, area: Rect, _state: &AppState, input: &str) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(3), // Input
            Constraint::Min(0),    // Info
            Constraint::Length(3), // Help
        ])
        .split(area);

    // Title
    let title = Paragraph::new("Create New Group")
        .style(
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    // Input
    let input_widget = Paragraph::new(input)
        .style(Style::default().fg(Color::Yellow))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Group Name"),
        );
    f.render_widget(input_widget, chunks[1]);

    // Info
    let info = Paragraph::new(vec![
        Line::from(""),
        Line::from("Enter a name for your new group."),
        Line::from("After creating, you can add nodes to it."),
    ])
    .alignment(Alignment::Center);
    f.render_widget(info, chunks[2]);

    // Help
    let help = Paragraph::new(Line::from(vec![
        Span::styled("Enter", Style::default().fg(Color::Yellow)),
        Span::raw(" Create  "),
        Span::styled("ESC", Style::default().fg(Color::Yellow)),
        Span::raw(" Cancel"),
    ]))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(help, chunks[3]);
}

fn render_node_selection(
    f: &mut Frame,
    area: Rect,
    state: &AppState,
    config: &AppConfig,
    group_name: &str,
    selected_index: usize,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(0),    // Node list
            Constraint::Length(3), // Help
        ])
        .split(area);

    // Title
    let title = Paragraph::new(format!("Add Nodes to Group: {}", group_name))
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    // Get all available nodes from routes
    let routes = crate::clash::HumanRoute::from_proxies(&state.clash_state.proxies, state.mode);
    let mut all_nodes = Vec::new();
    for route in routes {
        all_nodes.extend(route.all_nodes);
    }
    all_nodes.sort();
    all_nodes.dedup();

    // Get nodes already in the group
    let group_nodes = config.get_group_nodes(group_name).cloned().unwrap_or_default();

    let items: Vec<ListItem> = all_nodes
        .iter()
        .enumerate()
        .map(|(idx, node)| {
            let in_group = group_nodes.contains(node);
            let line = if idx == selected_index {
                Line::from(vec![
                    Span::styled("▶ ", Style::default().fg(Color::Yellow)),
                    if in_group {
                        Span::styled("[✓] ", Style::default().fg(Color::Green))
                    } else {
                        Span::raw("[ ] ")
                    },
                    Span::styled(
                        node,
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])
            } else {
                Line::from(vec![
                    Span::raw("  "),
                    if in_group {
                        Span::styled("[✓] ", Style::default().fg(Color::Green))
                    } else {
                        Span::raw("[ ] ")
                    },
                    Span::raw(node),
                ])
            };
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!("Available Nodes ({})", all_nodes.len())),
    );
    f.render_widget(list, chunks[1]);

    // Help
    let help = Paragraph::new(Line::from(vec![
        Span::styled("Space", Style::default().fg(Color::Yellow)),
        Span::raw(" Toggle  "),
        Span::styled("Enter", Style::default().fg(Color::Yellow)),
        Span::raw(" Done  "),
        Span::styled("ESC", Style::default().fg(Color::Yellow)),
        Span::raw(" Cancel"),
    ]))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(help, chunks[2]);
}

fn render_group_view(
    f: &mut Frame,
    area: Rect,
    _state: &AppState,
    config: &AppConfig,
    group_name: &str,
    selected_index: usize,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(0),    // Nodes in group
            Constraint::Length(3), // Help
        ])
        .split(area);

    // Title
    let title = Paragraph::new(format!("Group: {}", group_name))
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    // Nodes in group
    let nodes = config.get_group_nodes(group_name).cloned().unwrap_or_default();
    let items: Vec<ListItem> = nodes
        .iter()
        .enumerate()
        .map(|(idx, node)| {
            let line = if idx == selected_index {
                Line::from(vec![
                    Span::styled("▶ ", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        node,
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])
            } else {
                Line::from(vec![Span::raw("  "), Span::raw(node)])
            };
            ListItem::new(line)
        })
        .collect();

    let list_title = if nodes.is_empty() {
        "Nodes (Empty)".to_string()
    } else {
        format!("Nodes ({} total)", nodes.len())
    };

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(list_title));
    f.render_widget(list, chunks[1]);

    // Help
    let help = Paragraph::new(Line::from(vec![
        Span::styled("a", Style::default().fg(Color::Yellow)),
        Span::raw(" Add Nodes  "),
        Span::styled("d", Style::default().fg(Color::Yellow)),
        Span::raw(" Remove Node  "),
        Span::styled("ESC", Style::default().fg(Color::Yellow)),
        Span::raw(" Back"),
    ]))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(help, chunks[2]);
}
