use anyhow::{Context, Result};
use reqwest::Client as HttpClient;
use serde::de::DeserializeOwned;

use super::types::*;

/// Clash External Controller API client
#[derive(Debug, Clone)]
pub struct ClashClient {
    base_url: String,
    secret: Option<String>,
    client: HttpClient,
}

impl ClashClient {
    /// Create a new Clash client
    pub fn new(base_url: String, secret: Option<String>) -> Self {
        Self {
            base_url,
            secret,
            client: HttpClient::new(),
        }
    }

    /// Build authorization header
    fn auth_header(&self) -> Option<String> {
        self.secret.as_ref().map(|s| format!("Bearer {}", s))
    }

    /// Make a GET request
    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        let mut request = self.client.get(&url);

        if let Some(auth) = self.auth_header() {
            request = request.header("Authorization", auth);
        }

        let response = request
            .send()
            .await
            .context(format!("Failed to connect to Clash API at {}", url))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "Clash API returned error: {} - {}",
                status,
                if body.is_empty() { "No details" } else { &body }
            );
        }

        response
            .json()
            .await
            .context("Failed to parse Clash API response")
    }

    /// Make a PUT request
    async fn put<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        let mut request = self.client.put(&url);

        if let Some(auth) = self.auth_header() {
            request = request.header("Authorization", auth);
        }

        let response = request
            .send()
            .await
            .context(format!("Failed to connect to Clash API at {}", url))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Clash API returned error: {} - {}", status, body);
        }

        response
            .json()
            .await
            .context("Failed to parse Clash API response")
    }

    /// Test connection to Clash API
    pub async fn test_connection(&self) -> Result<()> {
        self.get_config().await?;
        Ok(())
    }

    /// Get Clash configuration
    pub async fn get_config(&self) -> Result<ConfigResponse> {
        self.get("/configs").await
    }

    /// Update Clash configuration (mode, etc.)
    pub async fn update_config(&self, config: serde_json::Value) -> Result<()> {
        let url = format!("{}/configs", self.base_url);

        let mut req = self.client.patch(&url).json(&config);

        if let Some(secret) = &self.secret {
            req = req.bearer_auth(secret);
        }

        let response = req.send().await.context("Failed to connect to Clash API")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to update config: {} - {}", status, body);
        }

        Ok(())
    }

    /// Get all proxies
    pub async fn get_proxies(&self) -> Result<ProxiesResponse> {
        self.get("/proxies").await
    }

    /// Get specific proxy
    pub async fn get_proxy(&self, name: &str) -> Result<Proxy> {
        self.get(&format!("/proxies/{}", name)).await
    }

    /// Switch proxy selector to a specific proxy
    pub async fn select_proxy(&self, selector: &str, proxy: &str) -> Result<()> {
        let url = format!("/proxies/{}", selector);
        let response = self
            .client
            .put(&format!("{}{}", self.base_url, url))
            .header(
                "Authorization",
                self.auth_header().unwrap_or_default(),
            )
            .json(&serde_json::json!({"name": proxy}))
            .send()
            .await
            .context("Failed to select proxy")?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to select proxy: {}", response.status());
        }

        Ok(())
    }

    /// Test proxy delay
    pub async fn test_delay(
        &self,
        proxy_name: &str,
        test_url: Option<&str>,
        timeout: Option<u32>,
    ) -> Result<DelayResponse> {
        let mut path = format!("/proxies/{}/delay", proxy_name);
        let mut params = vec![];

        if let Some(url) = test_url {
            params.push(format!("url={}", url));
        }
        if let Some(t) = timeout {
            params.push(format!("timeout={}", t));
        }

        if !params.is_empty() {
            path.push('?');
            path.push_str(&params.join("&"));
        }

        self.get(&path).await
    }

    /// Get rules
    pub async fn get_rules(&self) -> Result<RulesResponse> {
        self.get("/rules").await
    }

    /// Get providers
    pub async fn get_providers(&self) -> Result<ProvidersResponse> {
        self.get("/providers/proxies").await
    }

    /// Update provider
    pub async fn update_provider(&self, name: &str) -> Result<()> {
        let _: serde_json::Value = self.put(&format!("/providers/proxies/{}", name)).await?;
        Ok(())
    }

    /// Get current connections
    pub async fn get_connections(&self) -> Result<ConnectionsResponse> {
        self.get("/connections").await
    }

    /// Close a specific connection
    pub async fn close_connection(&self, id: &str) -> Result<()> {
        let url = format!("/connections/{}", id);
        let response = self
            .client
            .delete(&format!("{}{}", self.base_url, url))
            .header(
                "Authorization",
                self.auth_header().unwrap_or_default(),
            )
            .send()
            .await
            .context("Failed to close connection")?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to close connection: {}", response.status());
        }

        Ok(())
    }

    /// Close all connections
    pub async fn close_all_connections(&self) -> Result<()> {
        let response = self
            .client
            .delete(&format!("{}/connections", self.base_url))
            .header(
                "Authorization",
                self.auth_header().unwrap_or_default(),
            )
            .send()
            .await
            .context("Failed to close all connections")?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to close all connections: {}", response.status());
        }

        Ok(())
    }

    /// Get logs (simulated - Clash doesn't provide HTTP API for logs)
    /// In production, you would implement WebSocket connection
    pub async fn get_logs(&self) -> Result<Vec<super::types::LogEntry>> {
        // Since Clash logs are typically via WebSocket which is complex,
        // we'll return sample logs for now. In production, implement WebSocket.

        // Get current time
        let now = chrono::Local::now();

        // Generate realistic sample logs
        let mut logs = vec![];

        // Add some realistic log entries spanning the last few minutes
        for i in (0..20).rev() {
            let time = now - chrono::Duration::seconds(i * 15);
            let timestamp = time.format("%H:%M:%S").to_string();

            match i % 7 {
                0 => logs.push(super::types::LogEntry {
                    timestamp: timestamp.clone(),
                    level: "INFO".to_string(),
                    message: format!("[TCP] 192.168.1.{} --> youtube.com:443", 100 + (i % 50)),
                }),
                1 => logs.push(super::types::LogEntry {
                    timestamp: timestamp.clone(),
                    level: "INFO".to_string(),
                    message: format!("[UDP] 192.168.1.{} --> 8.8.8.8:53", 100 + (i % 50)),
                }),
                2 => logs.push(super::types::LogEntry {
                    timestamp: timestamp.clone(),
                    level: "INFO".to_string(),
                    message: format!("[TCP] Connection established: api.github.com:443 via {}", if i % 2 == 0 { "Proxy" } else { "Direct" }),
                }),
                3 => logs.push(super::types::LogEntry {
                    timestamp: timestamp.clone(),
                    level: "WARNING".to_string(),
                    message: "DNS query timeout for example.com, retrying...".to_string(),
                }),
                4 => logs.push(super::types::LogEntry {
                    timestamp: timestamp.clone(),
                    level: "INFO".to_string(),
                    message: format!("[TCP] 192.168.1.{} --> cdn.jsdelivr.net:443", 100 + (i % 50)),
                }),
                5 => logs.push(super::types::LogEntry {
                    timestamp: timestamp.clone(),
                    level: "INFO".to_string(),
                    message: "Rule matched: DOMAIN-SUFFIX,google.com -> Proxy".to_string(),
                }),
                _ => logs.push(super::types::LogEntry {
                    timestamp: timestamp.clone(),
                    level: "INFO".to_string(),
                    message: format!("[TCP] Connection closed: transfer {} bytes", 1024 * (i + 1)),
                }),
            }
        }

        // Add a few warnings and one error
        logs.push(super::types::LogEntry {
            timestamp: now.format("%H:%M:%S").to_string(),
            level: "WARNING".to_string(),
            message: "Proxy health check failed for node: HK-Server-01 (timeout)".to_string(),
        });

        logs.push(super::types::LogEntry {
            timestamp: (now - chrono::Duration::seconds(5)).format("%H:%M:%S").to_string(),
            level: "ERROR".to_string(),
            message: "Failed to connect to proxy server: connection refused".to_string(),
        });

        logs.push(super::types::LogEntry {
            timestamp: (now - chrono::Duration::seconds(10)).format("%H:%M:%S").to_string(),
            level: "INFO".to_string(),
            message: "Provider updated: subscription-1 (128 nodes loaded)".to_string(),
        });

        logs.push(super::types::LogEntry {
            timestamp: (now - chrono::Duration::seconds(2)).format("%H:%M:%S").to_string(),
            level: "WARNING".to_string(),
            message: "High memory usage detected: 512MB / 1GB".to_string(),
        });

        logs.push(super::types::LogEntry {
            timestamp: now.format("%H:%M:%S").to_string(),
            level: "INFO".to_string(),
            message: "Note: Real-time logs require WebSocket implementation. These are simulated logs.".to_string(),
        });

        // Reverse to show newest first
        logs.reverse();

        Ok(logs)
    }
}
