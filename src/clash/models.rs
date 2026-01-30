use crate::app::Mode;
use crate::clash::{Proxy, ProxyType};
use std::collections::HashMap;

/// Human-friendly route representation
#[derive(Debug, Clone)]
pub struct HumanRoute {
    pub name: String,
    #[allow(dead_code)]
    pub proxy_type: ProxyType,
    pub current_node: Option<String>,
    pub all_nodes: Vec<String>,
    pub node_count: usize,
}

impl HumanRoute {
    /// Extract human routes from proxy map (always show all groups)
    pub fn from_proxies(proxies: &HashMap<String, Proxy>, _mode: Mode) -> Vec<Self> {
        let mut routes = Vec::new();

        for (name, proxy) in proxies {
            // Filter by proxy type - only show groups that user can interact with
            match proxy.proxy_type {
                ProxyType::Selector
                | ProxyType::Smart
                | ProxyType::URLTest
                | ProxyType::LoadBalance => {
                    let all_nodes = proxy.all.clone().unwrap_or_default();

                    // Show all groups including GLOBAL
                    // But skip completely empty groups (no point showing them)
                    if all_nodes.is_empty() && proxy.now.is_none() {
                        continue;
                    }

                    routes.push(HumanRoute {
                        name: name.clone(),
                        proxy_type: proxy.proxy_type.clone(),
                        current_node: proxy.now.clone(),
                        node_count: all_nodes.len(),
                        all_nodes,
                    });
                }
                _ => {
                    // Skip non-group types (Direct, Reject, individual proxies, etc.)
                }
            }
        }

        // Sort by name, but put GLOBAL first if it exists
        routes.sort_by(|a, b| {
            if a.name == "GLOBAL" {
                std::cmp::Ordering::Less
            } else if b.name == "GLOBAL" {
                std::cmp::Ordering::Greater
            } else {
                a.name.cmp(&b.name)
            }
        });

        routes
    }

    /// Get display name (truncate if too long)
    pub fn display_name(&self) -> String {
        if self.name.len() > 40 {
            format!("{}...", &self.name[..37])
        } else {
            self.name.clone()
        }
    }

    /// Get current node display
    pub fn current_display(&self) -> String {
        if let Some(node) = &self.current_node {
            if node.len() > 30 {
                format!("{}...", &node[..27])
            } else {
                node.clone()
            }
        } else {
            "None".to_string()
        }
    }
}
