use std::time::Instant;

use crate::app::Mode;
use crate::clash::{ClashClient, ClashMode, Proxy, ProxyType};
use crate::config::Preset;
use anyhow::Result;
use std::collections::HashMap;
use tokio::sync::mpsc;

/// Delay test result message
#[derive(Debug, Clone)]
pub struct DelayTestResult {
    pub node: String,
    pub delay: Option<u32>, // None if test failed
}

/// Delay test result
#[derive(Debug, Clone)]
pub struct DelayResult {
    pub delay: u32,
    #[allow(dead_code)]
    pub tested_at: Instant,
}

/// Global application state
#[derive(Debug)]
pub struct AppState {
    pub clash_state: ClashState,
    pub current_page: Page,
    pub mode: Mode,
    pub preset: Preset,
    pub status_message: Option<String>,
    pub delay_cache: HashMap<String, DelayResult>,
    pub testing_nodes: Vec<String>,
    pub delay_rx: mpsc::UnboundedReceiver<DelayTestResult>,
    delay_tx: mpsc::UnboundedSender<DelayTestResult>,
}

impl AppState {
    pub fn new(client: ClashClient, preset: Preset) -> Self {
        let (delay_tx, delay_rx) = mpsc::unbounded_channel();
        let mode = preset.default_mode();

        Self {
            clash_state: ClashState::new(client),
            current_page: Page::Home,
            mode,
            preset,
            status_message: None,
            delay_cache: HashMap::new(),
            testing_nodes: Vec::new(),
            delay_rx,
            delay_tx,
        }
    }

    /// Refresh Clash state from API
    pub async fn refresh(&mut self) -> Result<()> {
        self.clash_state.refresh().await
    }

    /// Select a proxy for a selector group
    pub async fn select_proxy(&mut self, selector: &str, proxy: &str) -> Result<()> {
        self.clash_state
            .client
            .select_proxy(selector, proxy)
            .await?;
        self.status_message = Some(format!("Switched {} to {}", selector, proxy));
        // Refresh to get updated state
        let _ = self.refresh().await;
        Ok(())
    }

    /// Test delay for a proxy (non-blocking)
    /// Starts background test, result will arrive via channel
    pub fn start_test_delay(&mut self, proxy: String) {
        // Mark as testing
        if !self.testing_nodes.contains(&proxy) {
            self.testing_nodes.push(proxy.clone());
        }

        // Clone what we need for the async task
        let client = self.clash_state.client.clone();
        let proxy_name = proxy.clone();
        let tx = self.delay_tx.clone();

        // Spawn background task
        tokio::spawn(async move {
            let result = client
                .test_delay(&proxy_name, Some("https://www.google.com"), Some(5000))
                .await;

            let delay = result.ok().map(|r| r.delay);

            // Send result back
            let _ = tx.send(DelayTestResult {
                node: proxy_name,
                delay,
            });
        });
    }

    /// Process any pending delay test results
    pub fn process_delay_results(&mut self) {
        while let Ok(result) = self.delay_rx.try_recv() {
            // Remove from testing list
            self.testing_nodes.retain(|n| n != &result.node);

            // Update cache if test succeeded
            if let Some(delay) = result.delay {
                self.delay_cache.insert(
                    result.node.clone(),
                    DelayResult {
                        delay,
                        tested_at: Instant::now(),
                    },
                );

                // Update status message
                let status = if delay < 200 {
                    "Fast"
                } else if delay < 500 {
                    "Good"
                } else {
                    "Slow"
                };
                self.status_message = Some(format!("{}: {}ms ({})", result.node, delay, status));
            } else {
                self.status_message = Some(format!("{}: Test failed", result.node));
            }
        }
    }

    /// Check if a node is currently being tested
    pub fn is_testing(&self, node: &str) -> bool {
        self.testing_nodes.contains(&node.to_string())
    }

    /// Get cached delay result for a node
    pub fn get_delay(&self, node: &str) -> Option<&DelayResult> {
        self.delay_cache.get(node)
    }

    /// Get current active node (from first available route)
    pub fn get_current_node(&self) -> Option<String> {
        // Try to find the first route with a current node
        let routes = crate::clash::HumanRoute::from_proxies(&self.clash_state.proxies, self.mode);
        for route in routes {
            if let Some(node) = route.current_node {
                return Some(node);
            }
        }
        None
    }

    /// Check if a node is testable (not Direct/Reject type)
    pub fn is_node_testable(&self, node_name: &str) -> bool {
        if let Some(proxy) = self.clash_state.proxies.get(node_name) {
            !matches!(
                proxy.proxy_type,
                ProxyType::Direct
                    | ProxyType::Reject
                    | ProxyType::RejectDrop
                    | ProxyType::Compatible
                    | ProxyType::Pass
            )
        } else {
            // If we can't find the proxy, assume it's testable
            // This handles cases where the node name might be in a nested structure
            true
        }
    }

    /// Switch Clash mode (Rule/Global/Direct)
    pub async fn switch_mode(&mut self, mode: ClashMode) -> Result<()> {
        let config = serde_json::json!({
            "mode": mode.as_str()
        });

        self.clash_state.client.update_config(config).await?;
        self.status_message = Some(format!("Switched to {} mode", mode.as_str()));
        // Refresh to get updated state
        let _ = self.refresh().await;
        Ok(())
    }

    /// Update all providers
    #[allow(dead_code)]
    pub async fn update_all_providers(&mut self) -> Result<()> {
        self.status_message = Some("Updating all providers...".to_string());

        // In a real implementation, we would:
        // 1. Get all providers
        // 2. Update each one
        // 3. Show progress

        // For now, just show a placeholder message
        self.status_message = Some("Provider update not yet implemented".to_string());

        Ok(())
    }
}

/// Current page
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Page {
    Home,
    Routes,
    Rules,
    Update,
    Connections,
    Settings,
    Logs,
    Performance,
}

/// Clash state from API
#[derive(Debug)]
pub struct ClashState {
    pub client: ClashClient,
    pub mode: ClashMode,
    pub proxies: HashMap<String, Proxy>,
    pub current_selector: Option<String>,
    pub current_proxy: Option<String>,
    pub last_update: Instant,
    pub error: Option<String>,
}

impl ClashState {
    pub fn new(client: ClashClient) -> Self {
        Self {
            client,
            mode: ClashMode::Rule,
            proxies: HashMap::new(),
            current_selector: None,
            current_proxy: None,
            last_update: Instant::now(),
            error: None,
        }
    }

    /// Refresh state from Clash API
    pub async fn refresh(&mut self) -> Result<()> {
        self.error = None;

        // Get config
        match self.client.get_config().await {
            Ok(config) => {
                if let Some(raw_mode) = config.mode.as_deref() {
                    if let Some(mode) = ClashMode::from_str(raw_mode) {
                        self.mode = mode;
                    }
                }
            }
            Err(e) => {
                self.error = Some(format!("Failed to get config: {}", e));
                return Err(e);
            }
        }

        // Get proxies
        match self.client.get_proxies().await {
            Ok(proxies_response) => {
                self.proxies = proxies_response.proxies;

                // Find the main selector (usually "GLOBAL" or first selector)
                self.find_main_selector();
            }
            Err(e) => {
                self.error = Some(format!("Failed to get proxies: {}", e));
                return Err(e);
            }
        }

        self.last_update = Instant::now();
        Ok(())
    }

    /// Find the main proxy selector
    fn find_main_selector(&mut self) {
        // Try to find "GLOBAL" first
        if self.proxies.contains_key("GLOBAL") {
            self.current_selector = Some("GLOBAL".to_string());
            if let Some(proxy) = self.proxies.get("GLOBAL") {
                self.current_proxy = proxy.now.clone();
            }
            return;
        }

        // Otherwise find the first selector
        for (name, proxy) in &self.proxies {
            if proxy.proxy_type == ProxyType::Selector {
                self.current_selector = Some(name.clone());
                self.current_proxy = proxy.now.clone();
                return;
            }
        }
    }

    /// Get health status based on proxy state
    pub fn get_health_status(&self) -> HealthStatus {
        if self.error.is_some() {
            return HealthStatus::Error;
        }

        if self.current_proxy.is_none() {
            return HealthStatus::Unknown;
        }

        // In a real implementation, we'd check delay history
        // For now, just return good if we have a proxy
        HealthStatus::Good
    }
}

/// Health status indicator
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HealthStatus {
    Good,
    #[allow(dead_code)]
    Fair,
    #[allow(dead_code)]
    Bad,
    Error,
    Unknown,
}

impl HealthStatus {
    pub fn as_str(&self) -> &str {
        match self {
            HealthStatus::Good => "Good",
            HealthStatus::Fair => "Fair",
            HealthStatus::Bad => "Bad",
            HealthStatus::Error => "Error",
            HealthStatus::Unknown => "Unknown",
        }
    }

    pub fn color(&self) -> ratatui::style::Color {
        use ratatui::style::Color;
        match self {
            HealthStatus::Good => Color::Green,
            HealthStatus::Fair => Color::Yellow,
            HealthStatus::Bad => Color::Red,
            HealthStatus::Error => Color::Magenta,
            HealthStatus::Unknown => Color::Gray,
        }
    }
}
