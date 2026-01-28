pub mod clash_config;
pub mod mihomo_party;
pub mod preset;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::ui::theme::Theme;
pub use clash_config::ClashConfig;
pub use preset::Preset;

/// Node group definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeGroup {
    pub name: String,
    pub nodes: Vec<String>,
}

/// clashctl application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Clash External Controller API URL
    pub api_url: String,

    /// Optional secret for authentication
    pub secret: Option<String>,

    /// Default mode (Simple or Expert)
    pub default_mode: String,

    /// Current preset name
    pub current_preset: String,

    /// Custom rules - whitelist (always proxy)
    #[serde(default)]
    pub whitelist: Vec<String>,

    /// Custom rules - blacklist (always direct)
    #[serde(default)]
    pub blacklist: Vec<String>,

    /// Favorite nodes for quick access
    #[serde(default)]
    pub favorite_nodes: Vec<String>,

    /// Custom node groups
    #[serde(default)]
    pub node_groups: HashMap<String, Vec<String>>,

    /// UI theme
    #[serde(default)]
    pub theme: String,

    /// Cached Clash config path (for subscriptions)
    #[serde(default)]
    pub clash_config_path: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            api_url: "http://127.0.0.1:9090".to_string(),
            secret: None,
            default_mode: "simple".to_string(),
            current_preset: "default".to_string(),
            whitelist: Vec::new(),
            blacklist: Vec::new(),
            favorite_nodes: Vec::new(),
            node_groups: HashMap::new(),
            theme: "dark".to_string(),
            clash_config_path: None,
        }
    }
}

impl AppConfig {
    /// Get the default config file path
    pub fn default_path() -> Result<PathBuf> {
        let config_dir =
            dirs::config_dir().ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?;

        let clashctl_dir = config_dir.join("clashctl");
        Ok(clashctl_dir.join("config.yaml"))
    }

    /// Load configuration from file
    pub fn load() -> Result<Self> {
        let path = Self::default_path()?;

        if !path.exists() {
            // Return default config if file doesn't exist
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(&path)?;
        let config: AppConfig = serde_yaml::from_str(&contents)?;
        Ok(config)
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<()> {
        let path = Self::default_path()?;

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let contents = serde_yaml::to_string(self)?;
        fs::write(&path, contents)?;

        Ok(())
    }

    /// Merge command line arguments into config
    pub fn merge_cli(&mut self, api_url: Option<String>, secret: Option<String>) {
        if let Some(url) = api_url {
            self.api_url = url;
        }

        if let Some(s) = secret {
            self.secret = Some(s);
        }
    }

    /// Update preset and save
    pub fn set_preset(&mut self, preset: &Preset) -> Result<()> {
        self.current_preset = preset.as_str().to_string();
        self.save()
    }

    /// Add domain to whitelist (always proxy)
    pub fn add_to_whitelist(&mut self, domain: String) -> Result<()> {
        if !self.whitelist.contains(&domain) {
            self.whitelist.push(domain);
            self.save()?;
        }
        Ok(())
    }

    /// Add domain to blacklist (always direct)
    pub fn add_to_blacklist(&mut self, domain: String) -> Result<()> {
        if !self.blacklist.contains(&domain) {
            self.blacklist.push(domain);
            self.save()?;
        }
        Ok(())
    }

    /// Remove domain from whitelist
    pub fn remove_from_whitelist(&mut self, domain: &str) -> Result<()> {
        self.whitelist.retain(|d| d != domain);
        self.save()
    }

    /// Remove domain from blacklist
    pub fn remove_from_blacklist(&mut self, domain: &str) -> Result<()> {
        self.blacklist.retain(|d| d != domain);
        self.save()
    }

    /// Add node to favorites
    pub fn add_favorite(&mut self, node: String) -> Result<()> {
        if !self.favorite_nodes.contains(&node) {
            self.favorite_nodes.push(node);
            self.save()?;
        }
        Ok(())
    }

    /// Remove node from favorites
    pub fn remove_favorite(&mut self, node: &str) -> Result<()> {
        self.favorite_nodes.retain(|n| n != node);
        self.save()
    }

    /// Check if node is favorited
    pub fn is_favorite(&self, node: &str) -> bool {
        self.favorite_nodes.contains(&node.to_string())
    }

    /// Export configuration to a specific path
    pub fn export_to(&self, path: &std::path::Path) -> Result<()> {
        // Ensure directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let contents = serde_yaml::to_string(self)?;
        fs::write(path, contents)?;

        Ok(())
    }

    /// Import configuration from a specific path
    pub fn import_from(path: &std::path::Path) -> Result<Self> {
        if !path.exists() {
            anyhow::bail!("Configuration file not found: {}", path.display());
        }

        let contents = fs::read_to_string(path)?;
        let config: AppConfig = serde_yaml::from_str(&contents)?;
        Ok(config)
    }

    /// Get a pretty-printed JSON representation of the config
    pub fn to_json_pretty(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Create a new node group
    pub fn create_group(&mut self, name: String, nodes: Vec<String>) -> Result<()> {
        if self.node_groups.contains_key(&name) {
            anyhow::bail!("Group '{}' already exists", name);
        }
        self.node_groups.insert(name, nodes);
        self.save()
    }

    /// Add node to a group
    pub fn add_node_to_group(&mut self, group_name: &str, node: String) -> Result<()> {
        let group = self
            .node_groups
            .get_mut(group_name)
            .ok_or_else(|| anyhow::anyhow!("Group '{}' not found", group_name))?;

        if !group.contains(&node) {
            group.push(node);
            self.save()?;
        }
        Ok(())
    }

    /// Remove node from a group
    pub fn remove_node_from_group(&mut self, group_name: &str, node: &str) -> Result<()> {
        let group = self
            .node_groups
            .get_mut(group_name)
            .ok_or_else(|| anyhow::anyhow!("Group '{}' not found", group_name))?;

        group.retain(|n| n != node);
        self.save()
    }

    /// Delete a group
    pub fn delete_group(&mut self, name: &str) -> Result<()> {
        self.node_groups.remove(name);
        self.save()
    }

    /// Get all group names
    pub fn get_group_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.node_groups.keys().cloned().collect();
        names.sort();
        names
    }

    /// Get nodes in a group
    pub fn get_group_nodes(&self, name: &str) -> Option<&Vec<String>> {
        self.node_groups.get(name)
    }

    /// Get current theme
    pub fn get_theme(&self) -> Theme {
        Theme::from_str(&self.theme)
    }

    /// Set theme
    pub fn set_theme(&mut self, theme: Theme) -> Result<()> {
        self.theme = theme.as_str().to_string();
        self.save()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.api_url, "http://127.0.0.1:9090");
        assert_eq!(config.default_mode, "simple");
        assert_eq!(config.current_preset, "default");
    }
}
