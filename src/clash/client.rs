use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use reqwest::Client as HttpClient;
use serde::de::DeserializeOwned;
use tokio::sync::{mpsc, watch};
use tokio_tungstenite::tungstenite::{client::IntoClientRequest, Message};
use tokio_tungstenite::connect_async;
use url::Url;

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
            .header("Authorization", self.auth_header().unwrap_or_default())
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
            .header("Authorization", self.auth_header().unwrap_or_default())
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
            .header("Authorization", self.auth_header().unwrap_or_default())
            .send()
            .await
            .context("Failed to close all connections")?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to close all connections: {}", response.status());
        }

        Ok(())
    }

    /// Stream logs via WebSocket and push entries into sender until shutdown.
    pub async fn stream_logs(
        &self,
        level: Option<&str>,
        mut shutdown: watch::Receiver<bool>,
        sender: mpsc::UnboundedSender<super::types::LogEntry>,
    ) -> Result<()> {
        let url = self.logs_ws_url(level)?;
        let mut request = url.into_client_request()?;
        if let Some(auth) = self.auth_header() {
            request
                .headers_mut()
                .insert("Authorization", auth.parse()?);
        }

        let (ws_stream, _) = connect_async(request)
            .await
            .context("Failed to connect to logs WebSocket")?;
        let (mut write, mut read) = ws_stream.split();

        loop {
            tokio::select! {
                _ = shutdown.changed() => {
                    let _ = write.send(Message::Close(None)).await;
                    break;
                }
                msg = read.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            if let Some(entry) = parse_ws_log(&text) {
                                let _ = sender.send(entry);
                            }
                        }
                        Some(Ok(Message::Binary(bin))) => {
                            if let Ok(text) = String::from_utf8(bin) {
                                if let Some(entry) = parse_ws_log(&text) {
                                    let _ = sender.send(entry);
                                }
                            }
                        }
                        Some(Ok(Message::Ping(payload))) => {
                            let _ = write.send(Message::Pong(payload)).await;
                        }
                        Some(Ok(Message::Close(_))) => break,
                        Some(Ok(_)) => {}
                        Some(Err(err)) => return Err(err.into()),
                        None => break,
                    }
                }
            }
        }

        Ok(())
    }

    fn logs_ws_url(&self, level: Option<&str>) -> Result<Url> {
        let mut url = Url::parse(&self.base_url)
            .context("Invalid base URL for logs WebSocket")?;

        match url.scheme() {
            "https" => url.set_scheme("wss").map_err(|_| anyhow::anyhow!("Invalid scheme"))?,
            "http" => url.set_scheme("ws").map_err(|_| anyhow::anyhow!("Invalid scheme"))?,
            "wss" | "ws" => {}
            _ => anyhow::bail!("Unsupported URL scheme: {}", url.scheme()),
        }

        url.set_path("/logs");
        if let Some(level) = level {
            url.set_query(Some(&format!("level={}", level)));
        }
        Ok(url)
    }
}

#[derive(Debug, serde::Deserialize)]
struct WsLogSimple {
    #[serde(rename = "type")]
    level: String,
    payload: String,
}

#[derive(Debug, serde::Deserialize)]
struct WsLogNested {
    #[serde(rename = "type")]
    kind: String,
    payload: WsLogSimple,
}

fn parse_ws_log(text: &str) -> Option<super::types::LogEntry> {
    let (level, message) = if let Ok(nested) = serde_json::from_str::<WsLogNested>(text) {
        if nested.kind.to_lowercase() == "log" {
            (nested.payload.level, nested.payload.payload)
        } else {
            return None;
        }
    } else if let Ok(simple) = serde_json::from_str::<WsLogSimple>(text) {
        (simple.level, simple.payload)
    } else {
        let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
        return Some(super::types::LogEntry {
            timestamp,
            level: "INFO".to_string(),
            message: text.to_string(),
        });
    };

    let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
    Some(super::types::LogEntry {
        timestamp,
        level: level.to_uppercase(),
        message,
    })
}
