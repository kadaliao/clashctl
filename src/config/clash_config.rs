use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Clash proxy provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClashProxyProvider {
    #[serde(rename = "type")]
    pub provider_type: String,
    pub url: Option<String>,
    pub path: Option<String>,
    pub interval: Option<u32>,
    #[serde(rename = "health-check", default)]
    pub health_check: Option<HealthCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub enable: Option<bool>,
    pub url: Option<String>,
    pub interval: Option<u32>,
}

/// Clash configuration (partial, only what we need)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClashConfig {
    #[serde(rename = "proxy-providers", default)]
    pub proxy_providers: HashMap<String, ClashProxyProvider>,
}

impl ClashConfig {
    /// Load Clash configuration from file
    pub fn load(path: &PathBuf) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: ClashConfig = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    /// Try to find Clash config in common locations
    pub fn find_config() -> Option<PathBuf> {
        let possible_paths = vec![
            dirs::home_dir()?.join(".config/clash/config.yaml"),
            dirs::home_dir()?.join(".config/mihomo/config.yaml"),
            PathBuf::from("/etc/clash/config.yaml"),
        ];

        for path in possible_paths {
            if path.exists() {
                return Some(path);
            }
        }

        None
    }

    /// Get all proxy providers with their URLs
    pub fn get_providers(&self) -> Vec<(String, String, Option<String>)> {
        self.proxy_providers
            .iter()
            .map(|(name, provider)| {
                let url = provider.url.clone().or_else(|| provider.path.clone());
                (name.clone(), provider.provider_type.clone(), url)
            })
            .collect()
    }
}
