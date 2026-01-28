pub mod pages;
pub mod theme;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::io;

use crate::app::{AppState, Page};
use crate::clash::{ClashClient, ConnectionsResponse};
use crate::config::{AppConfig, Preset};
use crate::ui::theme::Theme;

pub async fn run(api_url: String, secret: Option<String>, preset: Preset, config: &mut AppConfig) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create Clash client and app state
    let client = ClashClient::new(api_url, secret);
    let mut state = AppState::new(client, preset);

    // Initial refresh
    let _ = state.refresh().await;

    // Run app
    let result = run_app(&mut terminal, &mut state, config).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    state: &mut AppState,
    config: &mut AppConfig,
) -> Result<()> {
    let mut last_refresh = std::time::Instant::now();
    let refresh_interval = std::time::Duration::from_secs(5);
    let mut selected_route_index = 0;
    let mut rules_scroll_offset = 0;
    let mut routes_expanded = false;  // Whether viewing node list
    let mut selected_node_index = 0;
    let mut show_quit_confirmation = false;  // Whether showing quit confirmation dialog
    let mut rules_search_query = String::new();  // Search query for rules
    let mut rules_search_mode = false;  // Whether in search mode
    let mut rules_edit_mode = pages::RuleEditMode::None;  // Rule edit mode
    let mut rules_edit_input = String::new();  // Rule edit input
    let mut rules_selected_index = 0;  // Selected rule index in Simple mode
    let mut rules_list_focus = pages::RuleListFocus::Whitelist;  // Which list is focused in Simple mode
    let mut connections_data: Option<ConnectionsResponse> = None;  // Connections data
    let mut connections_selected_index = 0;  // Selected connection index
    let mut connections_scroll_offset = 0;  // Connections scroll offset
    let mut connections_last_refresh = std::time::Instant::now();
    let mut connections_search_query = String::new();  // Connections search query
    let mut connections_search_mode = false;  // Connections search mode
    let mut settings_action = pages::SettingsAction::None;  // Settings page action state
    let mut logs_data: Vec<crate::clash::LogEntry> = Vec::new();  // Logs data
    let mut logs_level_filter = pages::LogLevel::All;  // Logs level filter
    let mut logs_search_query = String::new();  // Logs search query
    let mut logs_search_mode = false;  // Logs search mode
    let mut logs_scroll_offset = 0;  // Logs scroll offset
    let mut performance_last_refresh = std::time::Instant::now();
    let mut performance_upload_total = 0u64;
    let mut performance_download_total = 0u64;
    let mut performance_upload_rate = 0u64;
    let mut performance_download_rate = 0u64;
    let mut performance_connection_count = 0usize;
    let mut update_providers: Vec<(String, String, Option<String>, usize, Option<String>)> = Vec::new();  // (name, type, url, proxy_count, updated_at)
    let mut update_selected_index = 0;
    let mut _update_last_refresh = std::time::Instant::now();
    let mut rules_data: Vec<crate::clash::Rule> = Vec::new();  // Rules data from API

    loop {
        // Process any pending delay test results
        state.process_delay_results();

        // Auto refresh every 5 seconds
        if last_refresh.elapsed() >= refresh_interval {
            let _ = state.refresh().await;
            last_refresh = std::time::Instant::now();
        }

        // Auto refresh connections every 2 seconds when on Connections page
        if state.current_page == Page::Connections {
            if connections_last_refresh.elapsed() >= std::time::Duration::from_secs(2) {
                match state.clash_state.client.get_connections().await {
                    Ok(data) => connections_data = Some(data),
                    Err(e) => state.status_message = Some(format!("Failed to fetch connections: {}", e)),
                }
                connections_last_refresh = std::time::Instant::now();
            }
        }

        // Auto refresh performance data every 5 seconds when on Performance page
        if state.current_page == Page::Performance {
            if performance_last_refresh.elapsed() >= std::time::Duration::from_secs(5) {
                match state.clash_state.client.get_connections().await {
                    Ok(data) => {
                        // Calculate rates based on previous totals
                        let elapsed_secs = performance_last_refresh.elapsed().as_secs();
                        if elapsed_secs > 0 {
                            performance_upload_rate = (data.upload_total.saturating_sub(performance_upload_total)) / elapsed_secs;
                            performance_download_rate = (data.download_total.saturating_sub(performance_download_total)) / elapsed_secs;
                        }
                        performance_upload_total = data.upload_total;
                        performance_download_total = data.download_total;
                        performance_connection_count = data.connections.len();
                    }
                    Err(e) => state.status_message = Some(format!("Failed to fetch performance data: {}", e)),
                }
                performance_last_refresh = std::time::Instant::now();
            }
        }

        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Header
                    Constraint::Min(0),    // Content
                ])
                .split(f.size());

            // Header
            let theme = config.get_theme();
            render_header(f, chunks[0], &theme);

            // Content based on current page
            match state.current_page {
                Page::Home => pages::render_home(f, chunks[1], state),
                Page::Routes => {
                    if routes_expanded {
                        pages::render_routes_with_nodes(f, chunks[1], state, config, selected_route_index, selected_node_index)
                    } else {
                        pages::render_routes(f, chunks[1], state, config, selected_route_index)
                    }
                }
                Page::Rules => pages::render_rules(f, chunks[1], state, rules_scroll_offset, &rules_search_query, rules_search_mode, rules_edit_mode, &rules_edit_input, config, rules_selected_index, &rules_data, rules_list_focus),
                Page::Update => pages::render_update(f, chunks[1], state, &update_providers, update_selected_index),
                Page::Connections => pages::render_connections(f, chunks[1], state, connections_data.as_ref(), connections_selected_index, connections_scroll_offset, &connections_search_query, connections_search_mode),
                Page::Settings => pages::render_settings(f, chunks[1], state, config, &settings_action),
                Page::Logs => pages::render_logs(f, chunks[1], state, &logs_data, logs_level_filter, &logs_search_query, logs_scroll_offset),
                Page::Performance => pages::render_performance(f, chunks[1], state, performance_upload_total, performance_download_total, performance_upload_rate, performance_download_rate, performance_connection_count),
            }

            // Render quit confirmation dialog if needed
            if show_quit_confirmation {
                render_quit_confirmation(f, f.size());
            }
        })?;

        // Handle input (non-blocking with timeout)
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                // Handle quit confirmation dialog first
                if show_quit_confirmation {
                    match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => return Ok(()),
                        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                            show_quit_confirmation = false;
                        }
                        _ => {}
                    }
                    continue;
                }

                // Handle key events based on current page
                match state.current_page {
                    Page::Home => match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            show_quit_confirmation = true;
                        }
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            show_quit_confirmation = true;
                        }
                        KeyCode::Char('c') => {
                            state.current_page = Page::Connections;
                            connections_selected_index = 0;
                            connections_scroll_offset = 0;
                            // Fetch connections immediately
                            match state.clash_state.client.get_connections().await {
                                Ok(data) => connections_data = Some(data),
                                Err(e) => state.status_message = Some(format!("Failed to fetch connections: {}", e)),
                            }
                            connections_last_refresh = std::time::Instant::now();
                        }
                        KeyCode::Char('r') => {
                            state.status_message = Some("Refreshing...".to_string());
                            let _ = state.refresh().await;
                            last_refresh = std::time::Instant::now();
                            state.status_message = Some("Refreshed successfully!".to_string());
                        }
                        KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            let current_theme = config.get_theme();
                            let next_theme = current_theme.next();
                            let _ = config.set_theme(next_theme);
                            state.status_message = Some(format!("Switched to {} theme", next_theme.name()));
                        }
                        // Note: 't' key for speed test is removed from Home page
                        KeyCode::Char('m') => {
                            // Switch to next mode (Rule -> Global -> Direct -> Rule)
                            let next_mode = state.clash_state.mode.next();
                            if let Err(e) = state.switch_mode(next_mode).await {
                                state.status_message = Some(format!("Failed to switch mode: {}", e));
                            }
                            last_refresh = std::time::Instant::now();
                        }
                        KeyCode::Char('g') => {
                            state.current_page = Page::Routes;
                            selected_route_index = 0;
                        }
                        KeyCode::Char('l') => {
                            state.current_page = Page::Rules;
                            rules_scroll_offset = 0;
                            // Fetch rules immediately
                            match state.clash_state.client.get_rules().await {
                                Ok(rules_response) => rules_data = rules_response.rules,
                                Err(e) => state.status_message = Some(format!("Failed to fetch rules: {}", e)),
                            }
                        }
                        KeyCode::Char('u') => {
                            state.current_page = Page::Update;
                            update_selected_index = 0;
                            // Load providers from both config file and API
                            update_providers.clear();

                            // Try to load from Clash config file first (to get URLs)
                            if let Some(config_path) = crate::config::ClashConfig::find_config() {
                                if let Ok(clash_config) = crate::config::ClashConfig::load(&config_path) {
                                    // Get provider info from config file
                                    for (name, ptype, url) in clash_config.get_providers() {
                                        // Try to get node count and update time from API
                                        match state.clash_state.client.get_providers().await {
                                            Ok(api_providers) => {
                                                if let Some(api_provider) = api_providers.providers.get(&name) {
                                                    let proxy_count = api_provider.proxies.len();
                                                    let updated_at = api_provider.updated_at.clone();
                                                    update_providers.push((name, ptype, url, proxy_count, updated_at));
                                                } else {
                                                    // Provider in config but not in API yet
                                                    update_providers.push((name, ptype, url, 0, None));
                                                }
                                            }
                                            Err(_) => {
                                                // API failed, just use config info
                                                update_providers.push((name, ptype, url, 0, None));
                                            }
                                        }
                                    }
                                } else {
                                    state.status_message = Some("Failed to load Clash config file".to_string());
                                }
                            } else {
                                state.status_message = Some("Clash config file not found".to_string());
                            }

                            update_providers.sort_by(|a, b| a.0.cmp(&b.0));
                            _update_last_refresh = std::time::Instant::now();
                        }
                        KeyCode::Char('s') => {
                            state.current_page = Page::Settings;
                            settings_action = pages::SettingsAction::None;
                        }
                        KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            state.preset = state.preset.next();
                            state.mode = state.preset.default_mode();
                            let _ = config.set_preset(&state.preset);
                            state.status_message = Some(format!("Switched to {} preset: {}", state.preset.name(), state.preset.description()));
                        }
                        KeyCode::Char('p') => {
                            state.current_page = Page::Performance;
                            // Fetch initial performance data
                            match state.clash_state.client.get_connections().await {
                                Ok(data) => {
                                    performance_upload_total = data.upload_total;
                                    performance_download_total = data.download_total;
                                    performance_connection_count = data.connections.len();
                                    performance_upload_rate = 0;
                                    performance_download_rate = 0;
                                }
                                Err(e) => state.status_message = Some(format!("Failed to fetch performance data: {}", e)),
                            }
                            performance_last_refresh = std::time::Instant::now();
                        }
                        KeyCode::Char('o') => {
                            state.current_page = Page::Logs;
                            logs_scroll_offset = 0;
                            // Fetch logs immediately
                            match state.clash_state.client.get_logs().await {
                                Ok(data) => logs_data = data,
                                Err(e) => state.status_message = Some(format!("Failed to fetch logs: {}", e)),
                            }
                        }
                        _ => {}
                    },
                    Page::Routes => {
                        let routes = crate::clash::HumanRoute::from_proxies(&state.clash_state.proxies, state.mode);

                        if !routes_expanded {
                            // Route list mode
                            let max_index = routes.len().saturating_sub(1);

                            match key.code {
                                KeyCode::Char('q') | KeyCode::Esc => {
                                    // Return to Home instead of quitting
                                    state.current_page = Page::Home;
                                }
                                KeyCode::Char('h') => state.current_page = Page::Home,
                                KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                    // Cycle to next preset
                                    state.preset = state.preset.next();
                                    state.status_message = Some(format!("Switched to {} preset: {}", state.preset.name(), state.preset.description()));
                                }
                                KeyCode::Up => {
                                    selected_route_index = selected_route_index.saturating_sub(1);
                                }
                                KeyCode::Down => {
                                    if selected_route_index < max_index {
                                        selected_route_index += 1;
                                    }
                                }
                                KeyCode::Enter | KeyCode::Right => {
                                    // Enter node selection mode
                                    if selected_route_index < routes.len() {
                                        routes_expanded = true;
                                        selected_node_index = 0;

                                        // Find current node index
                                        let route = &routes[selected_route_index];
                                        if let Some(current) = &route.current_node {
                                            if let Some(idx) = route.all_nodes.iter().position(|n| n == current) {
                                                selected_node_index = idx;
                                            }
                                        }
                                    }
                                }
                                KeyCode::Char('t') | KeyCode::Char('T') => {
                                    // Batch test all nodes in selected route (only if preset allows)
                                    if state.preset.show_speed_test() && selected_route_index < routes.len() {
                                        let route = &routes[selected_route_index];
                                        // Filter out non-testable nodes (Direct, Reject, etc.) silently
                                        let testable_nodes: Vec<String> = route.all_nodes.iter()
                                            .filter(|node| state.is_node_testable(node))
                                            .cloned()
                                            .collect();

                                        if !testable_nodes.is_empty() {
                                            state.status_message = Some(format!("Testing {} nodes in {}...", testable_nodes.len(), route.display_name()));
                                            for node in testable_nodes {
                                                state.start_test_delay(node);
                                            }
                                        }
                                        // Silently skip if no testable nodes
                                    } else if !state.preset.show_speed_test() {
                                        state.status_message = Some("Speed test disabled in current preset".to_string());
                                    }
                                }
                                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                    return Ok(())
                                }
                                _ => {}
                            }
                        } else {
                            // Node selection mode
                            if selected_route_index >= routes.len() {
                                routes_expanded = false;
                                continue;
                            }

                            let route = &routes[selected_route_index];
                            let max_node_index = route.all_nodes.len().saturating_sub(1);

                            match key.code {
                                KeyCode::Char('q') => {
                                    // Return to Home instead of quitting
                                    routes_expanded = false;
                                    state.current_page = Page::Home;
                                }
                                KeyCode::Esc | KeyCode::Left => {
                                    // Back to route list
                                    routes_expanded = false;
                                }
                                KeyCode::Char('h') => {
                                    routes_expanded = false;
                                    state.current_page = Page::Home;
                                }
                                KeyCode::Up => {
                                    selected_node_index = selected_node_index.saturating_sub(1);
                                }
                                KeyCode::Down => {
                                    if selected_node_index < max_node_index {
                                        selected_node_index += 1;
                                    }
                                }
                                KeyCode::Enter => {
                                    // Switch to selected node
                                    if selected_node_index < route.all_nodes.len() {
                                        let node = &route.all_nodes[selected_node_index];
                                        let selector = route.name.clone();

                                        if let Err(e) = state.select_proxy(&selector, node).await {
                                            state.status_message = Some(format!("Failed to switch: {}", e));
                                        }

                                        last_refresh = std::time::Instant::now();
                                        // Stay in node selection mode to see the change
                                    }
                                }
                                KeyCode::Char('t') | KeyCode::Char('T') => {
                                    // Batch test all nodes in this route (only if preset allows)
                                    if state.preset.show_speed_test() {
                                        // Filter out non-testable nodes (Direct, Reject, etc.) silently
                                        let testable_nodes: Vec<String> = route.all_nodes.iter()
                                            .filter(|node| state.is_node_testable(node))
                                            .cloned()
                                            .collect();

                                        if !testable_nodes.is_empty() {
                                            state.status_message = Some(format!("Testing {} nodes...", testable_nodes.len()));
                                            for node in testable_nodes {
                                                state.start_test_delay(node);
                                            }
                                        }
                                        // Silently skip if no testable nodes
                                    } else {
                                        state.status_message = Some("Speed test disabled in current preset".to_string());
                                    }
                                }
                                KeyCode::Char('*') => {
                                    // Toggle favorite for selected node
                                    if selected_node_index < route.all_nodes.len() {
                                        let node = &route.all_nodes[selected_node_index];
                                        if config.is_favorite(node) {
                                            if let Err(e) = config.remove_favorite(node) {
                                                state.status_message = Some(format!("Failed to remove favorite: {}", e));
                                            } else {
                                                state.status_message = Some(format!("Removed {} from favorites", node));
                                            }
                                        } else {
                                            if let Err(e) = config.add_favorite(node.clone()) {
                                                state.status_message = Some(format!("Failed to add favorite: {}", e));
                                            } else {
                                                state.status_message = Some(format!("Added {} to favorites", node));
                                            }
                                        }
                                    }
                                }
                                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                    return Ok(())
                                }
                                _ => {}
                            }
                        }
                    }
                    Page::Rules => {
                        // Handle edit mode input
                        if rules_edit_mode != pages::RuleEditMode::None {
                            match key.code {
                                KeyCode::Char(c) => {
                                    rules_edit_input.push(c);
                                }
                                KeyCode::Backspace => {
                                    rules_edit_input.pop();
                                }
                                KeyCode::Esc => {
                                    rules_edit_mode = pages::RuleEditMode::None;
                                    rules_edit_input.clear();
                                }
                                KeyCode::Enter => {
                                    if !rules_edit_input.is_empty() {
                                        let result = match rules_edit_mode {
                                            pages::RuleEditMode::AddWhitelist => {
                                                config.add_to_whitelist(rules_edit_input.clone())
                                            }
                                            pages::RuleEditMode::AddBlacklist => {
                                                config.add_to_blacklist(rules_edit_input.clone())
                                            }
                                            pages::RuleEditMode::None => Ok(()),
                                        };

                                        if let Err(e) = result {
                                            state.status_message = Some(format!("Failed to save rule: {}", e));
                                        } else {
                                            state.status_message = Some(format!("Rule added: {}", rules_edit_input));
                                        }
                                    }
                                    rules_edit_mode = pages::RuleEditMode::None;
                                    rules_edit_input.clear();
                                }
                                _ => {}
                            }
                        } else if rules_search_mode {
                            // Handle search mode input
                            match key.code {
                                KeyCode::Char(c) => {
                                    rules_search_query.push(c);
                                }
                                KeyCode::Backspace => {
                                    rules_search_query.pop();
                                }
                                KeyCode::Esc => {
                                    rules_search_mode = false;
                                    rules_search_query.clear();
                                }
                                KeyCode::Enter => {
                                    rules_search_mode = false;
                                }
                                _ => {}
                            }
                        } else {
                            // Normal mode key handling
                            match key.code {
                                KeyCode::Char('q') | KeyCode::Esc => {
                                    // Return to Home instead of quitting
                                    state.current_page = Page::Home;
                                }
                                KeyCode::Char('h') => state.current_page = Page::Home,
                                KeyCode::Char('r') => {
                                    // Refresh rules
                                    state.status_message = Some("Refreshing rules...".to_string());
                                    match state.clash_state.client.get_rules().await {
                                        Ok(rules_response) => {
                                            rules_data = rules_response.rules;
                                            state.status_message = Some(format!("Loaded {} rules", rules_data.len()));
                                        }
                                        Err(e) => state.status_message = Some(format!("Failed to refresh: {}", e)),
                                    }
                                }
                                KeyCode::Char('/') => {
                                    // Enter search mode
                                    rules_search_mode = true;
                                    rules_search_query.clear();
                                }
                                KeyCode::Char('w') | KeyCode::Char('W') => {
                                    // Add to whitelist
                                    rules_edit_mode = pages::RuleEditMode::AddWhitelist;
                                    rules_edit_input.clear();
                                }
                                KeyCode::Char('b') | KeyCode::Char('B') => {
                                    // Add to blacklist
                                    rules_edit_mode = pages::RuleEditMode::AddBlacklist;
                                    rules_edit_input.clear();
                                }
                                KeyCode::Char('d') | KeyCode::Char('D') => {
                                    // Delete selected rule
                                    let result = match rules_list_focus {
                                        pages::RuleListFocus::Whitelist => {
                                            if rules_selected_index < config.whitelist.len() {
                                                let domain = config.whitelist[rules_selected_index].clone();
                                                config.remove_from_whitelist(&domain)
                                            } else {
                                                Ok(())
                                            }
                                        }
                                        pages::RuleListFocus::Blacklist => {
                                            if rules_selected_index < config.blacklist.len() {
                                                let domain = config.blacklist[rules_selected_index].clone();
                                                config.remove_from_blacklist(&domain)
                                            } else {
                                                Ok(())
                                            }
                                        }
                                        };

                                    if let Err(e) = result {
                                        state.status_message = Some(format!("Failed to delete rule: {}", e));
                                    } else {
                                        state.status_message = Some("Rule deleted".to_string());
                                        // Adjust selected index if needed
                                        let list_len = match rules_list_focus {
                                            pages::RuleListFocus::Whitelist => config.whitelist.len(),
                                            pages::RuleListFocus::Blacklist => config.blacklist.len(),
                                        };
                                        if rules_selected_index >= list_len && list_len > 0 {
                                            rules_selected_index = list_len - 1;
                                        }
                                    }
                                }
                                KeyCode::Up => {
                                    rules_scroll_offset = rules_scroll_offset.saturating_sub(1);
                                }
                                KeyCode::Down => {
                                    rules_scroll_offset = rules_scroll_offset.saturating_add(1);
                                }
                                KeyCode::Left => {
                                    rules_list_focus = pages::RuleListFocus::Whitelist;
                                    rules_selected_index = 0;
                                }
                                KeyCode::Right => {
                                    rules_list_focus = pages::RuleListFocus::Blacklist;
                                    rules_selected_index = 0;
                                }
                                KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                    state.preset = state.preset.next();
                                    let _ = config.set_preset(&state.preset);
                                    state.status_message = Some(format!("Switched to {} preset: {}", state.preset.name(), state.preset.description()));
                                }
                                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                    show_quit_confirmation = true;
                                }
                                _ => {}
                            }
                        }
                    }
                    Page::Update => {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => {
                                // Return to Home instead of quitting
                                state.current_page = Page::Home;
                            }
                            KeyCode::Char('h') => state.current_page = Page::Home,
                            KeyCode::Char('l') => {
                                state.current_page = Page::Rules;
                                rules_scroll_offset = 0;
                            }
                            KeyCode::Char('r') => {
                                // Refresh provider list
                                state.status_message = Some("Refreshing providers...".to_string());
                                update_providers.clear();

                                // Try to load from Clash config file first (to get URLs)
                                if let Some(config_path) = crate::config::ClashConfig::find_config() {
                                    if let Ok(clash_config) = crate::config::ClashConfig::load(&config_path) {
                                        // Get provider info from config file
                                        for (name, ptype, url) in clash_config.get_providers() {
                                            // Try to get node count and update time from API
                                            match state.clash_state.client.get_providers().await {
                                                Ok(api_providers) => {
                                                    if let Some(api_provider) = api_providers.providers.get(&name) {
                                                        let proxy_count = api_provider.proxies.len();
                                                        let updated_at = api_provider.updated_at.clone();
                                                        update_providers.push((name, ptype, url, proxy_count, updated_at));
                                                    } else {
                                                        update_providers.push((name, ptype, url, 0, None));
                                                    }
                                                }
                                                Err(_) => {
                                                    update_providers.push((name, ptype, url, 0, None));
                                                }
                                            }
                                        }
                                        update_providers.sort_by(|a, b| a.0.cmp(&b.0));
                                        state.status_message = Some("Providers refreshed!".to_string());
                                    } else {
                                        state.status_message = Some("Failed to load Clash config file".to_string());
                                    }
                                } else {
                                    state.status_message = Some("Clash config file not found".to_string());
                                }
                                _update_last_refresh = std::time::Instant::now();
                            }
                            KeyCode::Up => {
                                update_selected_index = update_selected_index.saturating_sub(1);
                            }
                            KeyCode::Down => {
                                let max_idx = update_providers.len().saturating_sub(1);
                                if update_selected_index < max_idx {
                                    update_selected_index += 1;
                                }
                            }
                            KeyCode::Enter => {
                                // Update selected provider
                                if update_selected_index < update_providers.len() {
                                    let provider_name = update_providers[update_selected_index].0.clone();
                                    state.status_message = Some(format!("Updating {}...", provider_name));
                                    match state.clash_state.client.update_provider(&provider_name).await {
                                        Ok(_) => {
                                            state.status_message = Some(format!("Updated {} successfully!", provider_name));
                                        }
                                        Err(e) => {
                                            state.status_message = Some(format!("Failed to update {}: {}", provider_name, e));
                                        }
                                    }
                                }
                            }
                            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                state.preset = state.preset.next();
                                state.mode = state.preset.default_mode();
                                state.status_message = Some(format!("Switched to {} preset: {}", state.preset.name(), state.preset.description()));
                            }
                            KeyCode::Char('u') => {
                                // Update all providers
                                state.status_message = Some("Updating all providers...".to_string());
                                let mut success_count = 0;
                                let mut fail_count = 0;

                                for (provider_name, _, _, _, _) in &update_providers {
                                    match state.clash_state.client.update_provider(provider_name).await {
                                        Ok(_) => success_count += 1,
                                        Err(_) => fail_count += 1,
                                    }
                                }

                                if fail_count == 0 {
                                    state.status_message = Some(format!("All {} providers updated successfully!", success_count));
                                } else {
                                    state.status_message = Some(format!("Updated: {} succeeded, {} failed", success_count, fail_count));
                                }
                            }
                            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                return Ok(())
                            }
                            _ => {}
                        }
                    }
                    Page::Connections => {
                        if connections_search_mode {
                            // Handle search mode input
                            match key.code {
                                KeyCode::Char(c) => {
                                    connections_search_query.push(c);
                                }
                                KeyCode::Backspace => {
                                    connections_search_query.pop();
                                }
                                KeyCode::Esc => {
                                    connections_search_mode = false;
                                    connections_search_query.clear();
                                }
                                KeyCode::Enter => {
                                    connections_search_mode = false;
                                }
                                _ => {}
                            }
                        } else {
                            // Normal mode
                            match key.code {
                                KeyCode::Char('q') | KeyCode::Esc => {
                                    // Return to Home instead of quitting
                                    state.current_page = Page::Home;
                                }
                                KeyCode::Char('h') => state.current_page = Page::Home,
                                KeyCode::Char('/') => {
                                    // Enter search mode
                                    connections_search_mode = true;
                                    connections_search_query.clear();
                                }
                                KeyCode::Char('r') => {
                                    // Refresh connections
                                    state.status_message = Some("Refreshing connections...".to_string());
                                    match state.clash_state.client.get_connections().await {
                                        Ok(data) => {
                                            connections_data = Some(data);
                                            state.status_message = Some("Connections refreshed!".to_string());
                                        }
                                        Err(e) => {
                                            state.status_message = Some(format!("Failed to refresh: {}", e));
                                        }
                                    }
                                    connections_last_refresh = std::time::Instant::now();
                                }
                                KeyCode::Up => {
                                    connections_selected_index = connections_selected_index.saturating_sub(1);
                                    // Adjust scroll if selection goes above visible area
                                    if connections_selected_index < connections_scroll_offset {
                                        connections_scroll_offset = connections_selected_index;
                                    }
                                }
                                KeyCode::Down => {
                                    if let Some(conn) = &connections_data {
                                        let max_index = conn.connections.len().saturating_sub(1);
                                        if connections_selected_index < max_index {
                                            connections_selected_index += 1;
                                            // Adjust scroll if selection goes below visible area
                                            // Assuming visible area height is ~15 items (each connection takes 2 lines)
                                            let visible_items = 7;
                                            if connections_selected_index >= connections_scroll_offset + visible_items {
                                                connections_scroll_offset = connections_selected_index - visible_items + 1;
                                            }
                                        }
                                    }
                                }
                                KeyCode::Char('d') | KeyCode::Char('D') => {
                                    // Close selected connection
                                    if let Some(conn) = &connections_data {
                                        if connections_selected_index < conn.connections.len() {
                                            let connection_id = conn.connections[connections_selected_index].id.clone();
                                            state.status_message = Some(format!("Closing connection {}...", connection_id));
                                            match state.clash_state.client.close_connection(&connection_id).await {
                                                Ok(_) => {
                                                    state.status_message = Some("Connection closed!".to_string());
                                                    // Refresh connections
                                                    if let Ok(data) = state.clash_state.client.get_connections().await {
                                                        connections_data = Some(data);
                                                        // Adjust selected index if needed
                                                        if let Some(conn) = &connections_data {
                                                            if connections_selected_index >= conn.connections.len() && conn.connections.len() > 0 {
                                                                connections_selected_index = conn.connections.len() - 1;
                                                            }
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    state.status_message = Some(format!("Failed to close connection: {}", e));
                                                }
                                            }
                                            connections_last_refresh = std::time::Instant::now();
                                        }
                                    }
                                }
                                KeyCode::Char('a') | KeyCode::Char('A') => {
                                    // Close all connections
                                    state.status_message = Some("Closing all connections...".to_string());
                                    match state.clash_state.client.close_all_connections().await {
                                        Ok(_) => {
                                            state.status_message = Some("All connections closed!".to_string());
                                            // Refresh connections
                                            if let Ok(data) = state.clash_state.client.get_connections().await {
                                                connections_data = Some(data);
                                                connections_selected_index = 0;
                                            }
                                        }
                                        Err(e) => {
                                            state.status_message = Some(format!("Failed to close all connections: {}", e));
                                        }
                                    }
                                    connections_last_refresh = std::time::Instant::now();
                                }
                                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                    show_quit_confirmation = true;
                                }
                                _ => {}
                            }
                        }
                    }
                    Page::Settings => {
                        match &settings_action {
                            pages::SettingsAction::ExportPrompt => {
                                match key.code {
                                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                                        // Export configuration
                                        let export_path = dirs::config_dir()
                                            .map(|p| p.join("clashctl/clashctl-export.yaml"))
                                            .unwrap_or_else(|| std::path::PathBuf::from("clashctl-export.yaml"));

                                        match config.export_to(&export_path) {
                                            Ok(_) => {
                                                settings_action = pages::SettingsAction::ExportSuccess(
                                                    export_path.display().to_string()
                                                );
                                            }
                                            Err(e) => {
                                                settings_action = pages::SettingsAction::Error(
                                                    format!("Export failed: {}", e)
                                                );
                                            }
                                        }
                                    }
                                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                                        settings_action = pages::SettingsAction::None;
                                    }
                                    _ => {}
                                }
                            }
                            pages::SettingsAction::ImportPrompt => {
                                match key.code {
                                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                                        // Import configuration
                                        let import_path = dirs::config_dir()
                                            .map(|p| p.join("clashctl/clashctl-import.yaml"))
                                            .unwrap_or_else(|| std::path::PathBuf::from("clashctl-import.yaml"));

                                        match AppConfig::import_from(&import_path) {
                                            Ok(imported_config) => {
                                                // Save imported config
                                                if let Err(e) = imported_config.save() {
                                                    settings_action = pages::SettingsAction::Error(
                                                        format!("Failed to save imported config: {}", e)
                                                    );
                                                } else {
                                                    *config = imported_config;
                                                    settings_action = pages::SettingsAction::ImportSuccess;
                                                }
                                            }
                                            Err(e) => {
                                                settings_action = pages::SettingsAction::Error(
                                                    format!("Import failed: {}", e)
                                                );
                                            }
                                        }
                                    }
                                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                                        settings_action = pages::SettingsAction::None;
                                    }
                                    _ => {}
                                }
                            }
                            _ => {
                                // Normal settings page navigation
                                match key.code {
                                    KeyCode::Char('q') | KeyCode::Esc => {
                                        state.current_page = Page::Home;
                                        settings_action = pages::SettingsAction::None;
                                    }
                                    KeyCode::Char('h') => {
                                        state.current_page = Page::Home;
                                        settings_action = pages::SettingsAction::None;
                                    }
                                    KeyCode::Char('e') | KeyCode::Char('E') => {
                                        settings_action = pages::SettingsAction::ExportPrompt;
                                    }
                                    KeyCode::Char('i') | KeyCode::Char('I') => {
                                        settings_action = pages::SettingsAction::ImportPrompt;
                                    }
                                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                        show_quit_confirmation = true;
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    Page::Logs => {
                        if logs_search_mode {
                            // Handle search mode input
                            match key.code {
                                KeyCode::Char(c) => {
                                    logs_search_query.push(c);
                                }
                                KeyCode::Backspace => {
                                    logs_search_query.pop();
                                }
                                KeyCode::Esc => {
                                    logs_search_mode = false;
                                    logs_search_query.clear();
                                }
                                KeyCode::Enter => {
                                    logs_search_mode = false;
                                }
                                _ => {}
                            }
                        } else {
                            // Normal mode
                            match key.code {
                                KeyCode::Char('q') | KeyCode::Esc => {
                                    state.current_page = Page::Home;
                                }
                                KeyCode::Char('h') => state.current_page = Page::Home,
                                KeyCode::Char('r') => {
                                    // Refresh logs
                                    state.status_message = Some("Refreshing logs...".to_string());
                                    match state.clash_state.client.get_logs().await {
                                        Ok(data) => {
                                            logs_data = data;
                                            state.status_message = Some("Logs refreshed!".to_string());
                                        }
                                        Err(e) => {
                                            state.status_message = Some(format!("Failed to refresh: {}", e));
                                        }
                                    }
                                }
                                KeyCode::Char('f') | KeyCode::Char('F') => {
                                    // Change filter level
                                    logs_level_filter = logs_level_filter.next();
                                    logs_scroll_offset = 0;
                                    state.status_message = Some(format!("Filter: {}", logs_level_filter.as_str()));
                                }
                                KeyCode::Char('/') => {
                                    // Enter search mode
                                    logs_search_mode = true;
                                    logs_search_query.clear();
                                }
                                KeyCode::Up => {
                                    logs_scroll_offset = logs_scroll_offset.saturating_sub(1);
                                }
                                KeyCode::Down => {
                                    logs_scroll_offset = logs_scroll_offset.saturating_add(1);
                                }
                                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                    show_quit_confirmation = true;
                                }
                                _ => {}
                            }
                        }
                    }
                    Page::Performance => {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => {
                                state.current_page = Page::Home;
                            }
                            KeyCode::Char('h') => state.current_page = Page::Home,
                            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                show_quit_confirmation = true;
                            }
                            KeyCode::Char('c') => {
                                // Navigate to Connections page
                                state.current_page = Page::Connections;
                                connections_selected_index = 0;
                                connections_scroll_offset = 0;
                                // Fetch connections immediately
                                match state.clash_state.client.get_connections().await {
                                    Ok(data) => connections_data = Some(data),
                                    Err(e) => state.status_message = Some(format!("Failed to fetch connections: {}", e)),
                                }
                                connections_last_refresh = std::time::Instant::now();
                            }
                            KeyCode::Char('r') => {
                                // Manual refresh
                                state.status_message = Some("Refreshing performance data...".to_string());
                                match state.clash_state.client.get_connections().await {
                                    Ok(data) => {
                                        let elapsed_secs = performance_last_refresh.elapsed().as_secs();
                                        if elapsed_secs > 0 {
                                            performance_upload_rate = (data.upload_total.saturating_sub(performance_upload_total)) / elapsed_secs;
                                            performance_download_rate = (data.download_total.saturating_sub(performance_download_total)) / elapsed_secs;
                                        }
                                        performance_upload_total = data.upload_total;
                                        performance_download_total = data.download_total;
                                        performance_connection_count = data.connections.len();
                                        state.status_message = Some("Performance data refreshed!".to_string());
                                    }
                                    Err(e) => {
                                        state.status_message = Some(format!("Failed to refresh: {}", e));
                                    }
                                }
                                performance_last_refresh = std::time::Instant::now();
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}

fn render_header(f: &mut ratatui::Frame, area: ratatui::layout::Rect, theme: &Theme) {
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            "clashctl",
            Style::default()
                .fg(theme.primary())
                .add_modifier(ratatui::style::Modifier::BOLD),
        ),
        Span::styled(
            " v0.1.0 - Simple-first TUI Clash Controller",
            Style::default().fg(theme.text()),
        ),
        Span::styled(
            format!(" [{}]", theme.name()),
            Style::default().fg(theme.text_muted()),
        ),
    ]))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(theme.border())));

    f.render_widget(header, area);
}

fn render_quit_confirmation(f: &mut ratatui::Frame, area: ratatui::layout::Rect) {
    // Create a centered dialog
    let dialog_width = 50;
    let dialog_height = 7;
    let x = (area.width.saturating_sub(dialog_width)) / 2;
    let y = (area.height.saturating_sub(dialog_height)) / 2;

    let dialog_area = ratatui::layout::Rect {
        x: x + area.x,
        y: y + area.y,
        width: dialog_width,
        height: dialog_height,
    };

    // Clear background
    let clear_block = Block::default()
        .style(Style::default().bg(Color::Black));
    f.render_widget(clear_block, dialog_area);

    // Dialog content
    let dialog_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Title
            Constraint::Length(1),  // Message
            Constraint::Length(1),  // Prompt
        ])
        .split(dialog_area);

    let title = Paragraph::new("Quit Confirmation")
        .style(Style::default().fg(Color::Yellow).add_modifier(ratatui::style::Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, dialog_chunks[0]);

    let message = Paragraph::new("Are you sure you want to quit?")
        .alignment(Alignment::Center);
    f.render_widget(message, dialog_chunks[1]);

    let prompt = Paragraph::new(Line::from(vec![
        Span::styled("Y", Style::default().fg(Color::Green).add_modifier(ratatui::style::Modifier::BOLD)),
        Span::raw("es / "),
        Span::styled("N", Style::default().fg(Color::Red).add_modifier(ratatui::style::Modifier::BOLD)),
        Span::raw("o"),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(prompt, dialog_chunks[2]);
}
