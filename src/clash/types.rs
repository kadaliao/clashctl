#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Clash mode
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ClashMode {
    Rule,
    Global,
    Direct,
}

impl ClashMode {
    pub fn as_str(&self) -> &str {
        match self {
            ClashMode::Rule => "rule",
            ClashMode::Global => "global",
            ClashMode::Direct => "direct",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            ClashMode::Rule => ClashMode::Global,
            ClashMode::Global => ClashMode::Direct,
            ClashMode::Direct => ClashMode::Rule,
        }
    }
}

/// Config response from GET /configs
#[derive(Debug, Clone, Deserialize)]
pub struct ConfigResponse {
    #[serde(default)]
    pub port: u16,
    #[serde(rename = "socks-port", default)]
    pub socks_port: u16,
    #[serde(rename = "redir-port", default)]
    pub redir_port: u16,
    #[serde(rename = "allow-lan", default)]
    pub allow_lan: bool,
    #[serde(default)]
    pub mode: Option<ClashMode>,
    #[serde(rename = "log-level", default)]
    pub log_level: String,
}

/// Proxy type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum ProxyType {
    Direct,
    Reject,
    #[serde(rename = "RejectDrop")]
    RejectDrop,
    Pass,
    Compatible,
    Shadowsocks,
    ShadowsocksR,
    Snell,
    Socks5,
    Http,
    Vmess,
    Vless,
    Trojan,
    Hysteria,
    Hysteria2,
    WireGuard,
    Tuic,
    Ssh,
    Selector,
    Fallback,
    #[serde(rename = "URLTest")]
    URLTest,
    LoadBalance,
    Relay,
    Smart,
    #[serde(other)]
    Unknown,
}

/// Proxy node or group
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Proxy {
    #[serde(rename = "type")]
    pub proxy_type: ProxyType,
    pub name: String,
    pub now: Option<String>,
    pub all: Option<Vec<String>>,
    pub history: Option<Vec<DelayHistory>>,
    pub udp: Option<bool>,
}

impl Default for Proxy {
    fn default() -> Self {
        Self {
            proxy_type: ProxyType::Direct,
            name: String::new(),
            now: None,
            all: None,
            history: None,
            udp: None,
        }
    }
}

/// Delay history
#[derive(Debug, Clone, Deserialize)]
pub struct DelayHistory {
    pub time: String,
    pub delay: u32,
    pub mean_delay: Option<u32>,
}

/// Proxies response from GET /proxies
#[derive(Debug, Clone, Deserialize)]
pub struct ProxiesResponse {
    pub proxies: HashMap<String, Proxy>,
}

/// Rule
#[derive(Debug, Clone, Deserialize)]
pub struct Rule {
    #[serde(rename = "type")]
    pub rule_type: String,
    pub payload: String,
    pub proxy: String,
}

/// Rules response from GET /rules
#[derive(Debug, Clone, Deserialize)]
pub struct RulesResponse {
    pub rules: Vec<Rule>,
}

/// Provider info
#[derive(Debug, Clone, Deserialize)]
pub struct Provider {
    pub name: String,
    #[serde(rename = "type")]
    pub provider_type: String,
    #[serde(rename = "vehicleType")]
    pub vehicle_type: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub proxies: Vec<Proxy>,
    #[serde(rename = "subscriptionInfo", default)]
    pub subscription_info: Option<SubscriptionInfo>,
}

/// Subscription info for a provider
#[derive(Debug, Clone, Deserialize)]
pub struct SubscriptionInfo {
    #[serde(default)]
    pub upload: u64,
    #[serde(default)]
    pub download: u64,
    #[serde(default)]
    pub total: u64,
    #[serde(default)]
    pub expire: u64,
}

/// Providers response from GET /providers/proxies
#[derive(Debug, Clone, Deserialize)]
pub struct ProvidersResponse {
    pub providers: HashMap<String, Provider>,
}

/// Delay test response from GET /proxies/:name/delay
#[derive(Debug, Clone, Deserialize)]
pub struct DelayResponse {
    pub delay: u32,
}

/// Connection metadata
#[derive(Debug, Clone, Deserialize)]
pub struct ConnectionMetadata {
    pub network: String,
    #[serde(rename = "type")]
    pub conn_type: String,
    #[serde(rename = "sourceIP")]
    pub source_ip: String,
    #[serde(rename = "destinationIP")]
    pub destination_ip: String,
    #[serde(rename = "sourcePort")]
    pub source_port: String,
    #[serde(rename = "destinationPort")]
    pub destination_port: String,
    pub host: Option<String>,
    #[serde(rename = "dnsMode")]
    pub dns_mode: Option<String>,
    #[serde(rename = "processPath")]
    pub process_path: Option<String>,
}

/// Connection info
#[derive(Debug, Clone, Deserialize)]
pub struct Connection {
    pub id: String,
    pub metadata: ConnectionMetadata,
    pub upload: u64,
    pub download: u64,
    pub start: String,
    pub chains: Vec<String>,
    pub rule: String,
    #[serde(rename = "rulePayload")]
    pub rule_payload: Option<String>,
}

/// Connections response from GET /connections
#[derive(Debug, Clone, Deserialize)]
pub struct ConnectionsResponse {
    #[serde(rename = "downloadTotal")]
    pub download_total: u64,
    #[serde(rename = "uploadTotal")]
    pub upload_total: u64,
    pub connections: Vec<Connection>,
}

/// Log entry (simulated - for HTTP API)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
}

#[derive(Debug, Clone)]
pub enum LogStreamStatus {
    Connected,
    Disconnected(String),
}

#[derive(Debug, Clone)]
pub enum LogStreamEvent {
    Entry(LogEntry),
    Status(LogStreamStatus),
}
