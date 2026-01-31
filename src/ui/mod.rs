pub mod pages;
pub mod theme;

use anyhow::Result;
use base64::Engine;
use chrono::{Local, TimeZone, Utc};
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
use std::fs::OpenOptions;
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;
use url::Url;

use crate::app::{AppState, Page};
use crate::clash::{ClashClient, ConnectionsResponse, LogEntry, LogStreamEvent, LogStreamStatus};
use crate::config::{mihomo_party, AppConfig, Preset};
use crate::ui::pages::update::{SubscriptionItem, SubscriptionSource};
use crate::ui::theme::Theme;

fn resolve_clash_config_path(config: &mut AppConfig) -> Option<PathBuf> {
    let hint = config.clash_config_path.as_deref().map(Path::new);
    let found = crate::config::ClashConfig::find_config_with_hint(hint);
    if let Some(path) = &found {
        if std::env::var_os("CLASH_CONFIG_PATH").is_none()
            && std::env::var_os("CLASH_PARTY_DIR").is_none()
        {
            let next_value = path.to_string_lossy().to_string();
            if config.clash_config_path.as_deref() != Some(next_value.as_str()) {
                config.clash_config_path = Some(next_value);
                let _ = config.save();
            }
        }
    }

    found
}

fn debug_log_path() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("CLASHCTL_DEBUG_LOG") {
        if !path.trim().is_empty() {
            return Some(PathBuf::from(path));
        }
    }
    if let Ok(enabled) = std::env::var("CLASHCTL_DEBUG") {
        let enabled = enabled.to_ascii_lowercase();
        if enabled == "1" || enabled == "true" || enabled == "yes" {
            return Some(PathBuf::from("/tmp/clashctl-debug.log"));
        }
    }
    None
}

fn debug_log(message: &str) {
    let path = match debug_log_path() {
        Some(path) => path,
        None => return,
    };
    let mut file = match OpenOptions::new().create(true).append(true).open(path) {
        Ok(file) => file,
        Err(_) => return,
    };
    let _ = writeln!(
        file,
        "[{}] {}",
        Local::now().format("%Y-%m-%d %H:%M:%S"),
        message
    );
}

fn format_timestamp_ms(timestamp_ms: i64) -> Option<String> {
    Local
        .timestamp_millis_opt(timestamp_ms)
        .single()
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
}

fn stop_logs_stream(
    logs_shutdown: &mut Option<watch::Sender<bool>>,
    logs_task: &mut Option<JoinHandle<()>>,
) {
    if let Some(tx) = logs_shutdown.take() {
        let _ = tx.send(true);
    }
    if let Some(handle) = logs_task.take() {
        handle.abort();
    }
}

fn start_logs_stream(
    client: ClashClient,
    level: Option<&str>,
    logs_tx: mpsc::UnboundedSender<LogStreamEvent>,
    logs_shutdown: &mut Option<watch::Sender<bool>>,
    logs_task: &mut Option<JoinHandle<()>>,
) {
    stop_logs_stream(logs_shutdown, logs_task);
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    *logs_shutdown = Some(shutdown_tx);
    let level = level.map(|value| value.to_string());
    *logs_task = Some(tokio::spawn(async move {
        if let Err(err) = client
            .stream_logs(level.as_deref(), shutdown_rx, logs_tx.clone())
            .await
        {
            let _ = logs_tx.send(LogStreamEvent::Status(LogStreamStatus::Disconnected(
                format!("error: {}", err),
            )));
            let _ = logs_tx.send(LogStreamEvent::Entry(LogEntry {
                timestamp: Local::now().format("%H:%M:%S").to_string(),
                level: "ERROR".to_string(),
                message: format!("Log stream error: {}", err),
            }));
        }
    }));
}

fn log_level_to_ws(level: pages::LogLevel) -> Option<&'static str> {
    match level {
        pages::LogLevel::All => None,
        pages::LogLevel::Info => Some("info"),
        pages::LogLevel::Warning => Some("warning"),
        pages::LogLevel::Error => Some("error"),
    }
}

#[derive(Debug, Clone)]
enum UpdateEvent {
    ItemFinished {
        index: usize,
        name: String,
        updated_at: Option<String>,
        success: bool,
        error: Option<String>,
    },
}

fn load_mihomo_party_subscriptions(config: &AppConfig) -> Result<Vec<SubscriptionItem>> {
    let hint = config.clash_config_path.as_deref().map(Path::new);
    let list_path = match mihomo_party::find_profile_list_with_hint(hint) {
        Some(path) => path,
        None => return Ok(Vec::new()),
    };

    let list = mihomo_party::MihomoPartyProfileList::load(&list_path)?;
    let current_id = list.current.clone();
    let mut items = Vec::new();

    for item in list.items {
        if item.url.is_none() {
            continue;
        }

        let profile_path = match mihomo_party::profile_path_from_list(&list_path, &item.id) {
            Some(path) => path,
            None => continue,
        };

        let proxy_count = mihomo_party::count_proxies_in_profile(&profile_path)
            .or_else(|| {
                std::fs::read(&profile_path)
                    .ok()
                    .map(|bytes| parse_raw_subscription(&bytes).len())
            })
            .unwrap_or(0);
        if proxy_count == 0 {
            debug_log(&format!(
                "subscription '{}' proxy_count=0 path={}",
                item.name,
                profile_path.display()
            ));
        }
        let updated_at = item.updated.and_then(format_timestamp_ms);

        items.push(SubscriptionItem {
            name: item.name,
            provider_type: format!("profile/{}", item.profile_type),
            url: item.url,
            proxy_count,
            updated_at,
            is_current: current_id.as_deref() == Some(item.id.as_str()),
            source: SubscriptionSource::MihomoPartyProfile {
                id: item.id,
                profile_path,
                list_path: list_path.clone(),
            },
        });
    }

    Ok(items)
}

async fn refresh_update_providers(
    state: &mut AppState,
    config: &mut AppConfig,
    update_providers: &mut Vec<SubscriptionItem>,
) {
    update_providers.clear();
    let mut loaded_any = false;

    match load_mihomo_party_subscriptions(config) {
        Ok(mut items) => {
            if !items.is_empty() {
                loaded_any = true;
                update_providers.append(&mut items);
            }
        }
        Err(_) => {
            state.status_message = Some("Failed to load Mihomo Party profiles".to_string());
        }
    }

    let config_path = resolve_clash_config_path(config);
    if let Some(config_path) = config_path {
        if let Ok(clash_config) = crate::config::ClashConfig::load(&config_path) {
            let api_providers = state.clash_state.client.get_providers().await.ok();

            for (name, ptype, url) in clash_config.get_providers() {
                let (proxy_count, updated_at) = if let Some(api) = &api_providers {
                    if let Some(api_provider) = api.providers.get(&name) {
                        (api_provider.proxies.len(), api_provider.updated_at.clone())
                    } else {
                        (0, None)
                    }
                } else {
                    (0, None)
                };

                update_providers.push(SubscriptionItem {
                    name: name.clone(),
                    provider_type: ptype,
                    url,
                    proxy_count,
                    updated_at,
                    is_current: false,
                    source: SubscriptionSource::ClashProvider { name },
                });
            }
        } else {
            state.status_message = Some("Failed to load Clash config file".to_string());
        }
    } else if !loaded_any {
        state.status_message = Some("Clash config file not found".to_string());
    }

    update_providers.sort_by(|a, b| a.name.cmp(&b.name));
}

async fn update_mihomo_party_profile(
    id: &str,
    url: &str,
    profile_path: &Path,
    list_path: &Path,
) -> Result<i64> {
    let response = reqwest::get(url).await?.error_for_status()?;
    let bytes = response.bytes().await?;
    debug_log(&format!(
        "update_profile id={} url_len={} bytes_len={}",
        id,
        url.len(),
        bytes.len()
    ));

    if let Some(parent) = profile_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let final_bytes = if looks_like_clash_config(&bytes) {
        debug_log("update_profile detected full config");
        bytes.to_vec()
    } else {
        debug_log("update_profile raw subscription, attempt convert");
        let work_config_path = mihomo_party::work_config_path_from_list(list_path);
        if let Some(work_config_path) = work_config_path {
            match convert_raw_subscription_to_config(&bytes, &work_config_path) {
                Ok((output, count)) => {
                    debug_log(&format!(
                        "update_profile converted raw -> config, proxies={}",
                        count
                    ));
                    output
                }
                Err(_) => bytes.to_vec(),
            }
        } else {
            bytes.to_vec()
        }
    };

    std::fs::write(profile_path, &final_bytes)?;

    let updated_at = Utc::now().timestamp_millis();
    mihomo_party::update_profile_updated_at(list_path, id, updated_at)?;

    Ok(updated_at)
}

fn spawn_update_task(
    update_tx: mpsc::UnboundedSender<UpdateEvent>,
    item: SubscriptionItem,
    index: usize,
    clash_client: ClashClient,
) {
    tokio::spawn(async move {
        let (success, updated_at, error) = match item.source {
            SubscriptionSource::ClashProvider { name } => {
                match clash_client.update_provider(&name).await {
                    Ok(_) => (true, None, None),
                    Err(e) => (false, None, Some(e.to_string())),
                }
            }
            SubscriptionSource::MihomoPartyProfile {
                id,
                profile_path,
                list_path,
            } => {
                let url = match item.url.as_deref() {
                    Some(url) => url,
                    None => {
                        let msg = "No URL for this subscription".to_string();
                        let _ = update_tx.send(UpdateEvent::ItemFinished {
                            index,
                            name: item.name,
                            updated_at: None,
                            success: false,
                            error: Some(msg),
                        });
                        return;
                    }
                };

                match update_mihomo_party_profile(&id, url, &profile_path, &list_path).await {
                    Ok(updated_at) => (true, format_timestamp_ms(updated_at), None),
                    Err(e) => (false, None, Some(e.to_string())),
                }
            }
        };

        let _ = update_tx.send(UpdateEvent::ItemFinished {
            index,
            name: item.name,
            updated_at,
            success,
            error,
        });
    });
}

fn is_http_url(raw: &str) -> bool {
    raw.starts_with("http://") || raw.starts_with("https://")
}

fn mapping_has_key(map: &serde_yaml::Mapping, key: &str) -> bool {
    map.contains_key(&serde_yaml::Value::String(key.to_string()))
}

fn looks_like_clash_config(bytes: &[u8]) -> bool {
    let value: serde_yaml::Value = match serde_yaml::from_slice(bytes) {
        Ok(value) => value,
        Err(_) => return false,
    };
    let map = match value.as_mapping() {
        Some(map) => map,
        None => return false,
    };

    mapping_has_key(map, "proxies")
        || mapping_has_key(map, "proxy-providers")
        || mapping_has_key(map, "proxy-groups")
        || mapping_has_key(map, "rules")
        || mapping_has_key(map, "rule-providers")
}

fn percent_decode(input: &str) -> String {
    let mut out: Vec<u8> = Vec::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = bytes[i + 1];
            let lo = bytes[i + 2];
            let hex = |b: u8| -> Option<u8> {
                match b {
                    b'0'..=b'9' => Some(b - b'0'),
                    b'a'..=b'f' => Some(b - b'a' + 10),
                    b'A'..=b'F' => Some(b - b'A' + 10),
                    _ => None,
                }
            };
            if let (Some(h), Some(l)) = (hex(hi), hex(lo)) {
                out.push((h << 4) | l);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

fn decode_base64(input: &str) -> Option<Vec<u8>> {
    let mut normalized: String = input.chars().filter(|c| !c.is_whitespace()).collect();
    normalized = normalized.replace('-', "+").replace('_', "/");
    while normalized.len() % 4 != 0 {
        normalized.push('=');
    }
    base64::engine::general_purpose::STANDARD
        .decode(normalized.as_bytes())
        .ok()
}

fn extract_subscription_lines(bytes: &[u8]) -> Vec<String> {
    let raw = String::from_utf8_lossy(bytes).trim().to_string();
    let mut candidates = vec![raw.clone()];
    if !raw.contains("://") {
        if let Some(decoded) = decode_base64(&raw) {
            if let Ok(decoded) = String::from_utf8(decoded) {
                candidates.push(decoded);
            }
        }
    }

    let text = candidates
        .into_iter()
        .find(|candidate| candidate.contains("://"))
        .unwrap_or(raw);

    text.lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect()
}

#[derive(Clone)]
struct ProxySpec {
    name: String,
    map: serde_yaml::Mapping,
}

fn parse_ss_url(line: &str) -> Option<ProxySpec> {
    let line = line.trim();
    if !line.starts_with("ss://") {
        return None;
    }
    let mut content = &line[5..];
    let mut name = None;
    if let Some(hash_idx) = content.find('#') {
        let (left, right) = content.split_at(hash_idx);
        content = left;
        name = Some(percent_decode(&right[1..]));
    }

    let mut plugin = None;
    let mut plugin_opts = None;
    if let Some(q_idx) = content.find('?') {
        let (left, right) = content.split_at(q_idx);
        content = left;
        let query = &right[1..];
        for (key, value) in url::form_urlencoded::parse(query.as_bytes()) {
            if key == "plugin" {
                let value = value.to_string();
                let mut parts = value.split(';');
                if let Some(first) = parts.next() {
                    if !first.is_empty() {
                        plugin = Some(first.to_string());
                    }
                }
                let rest: Vec<&str> = parts.collect();
                if !rest.is_empty() {
                    plugin_opts = Some(rest.join(";"));
                }
            }
        }
    }

    let mut userinfo = None;
    let mut hostport = None;
    if let Some(at_idx) = content.rfind('@') {
        userinfo = Some(content[..at_idx].to_string());
        hostport = Some(content[at_idx + 1..].to_string());
    } else {
        if let Some(decoded) = decode_base64(content) {
            if let Ok(decoded) = String::from_utf8(decoded) {
                if let Some(at_idx) = decoded.rfind('@') {
                    userinfo = Some(decoded[..at_idx].to_string());
                    hostport = Some(decoded[at_idx + 1..].to_string());
                }
            }
        }
    }

    let userinfo = userinfo?;
    let hostport = hostport?;
    let (cipher, password) = if userinfo.contains(':') {
        let mut parts = userinfo.splitn(2, ':');
        (parts.next()?.to_string(), parts.next()?.to_string())
    } else if let Some(decoded) = decode_base64(&userinfo) {
        let decoded = String::from_utf8(decoded).ok()?;
        let mut parts = decoded.splitn(2, ':');
        (parts.next()?.to_string(), parts.next()?.to_string())
    } else {
        return None;
    };

    let (server, port) = if hostport.starts_with('[') {
        let end = hostport.find(']')?;
        let host = hostport[1..end].to_string();
        let port_str = hostport.get(end + 2..)?;
        (host, port_str.parse::<u16>().ok()?)
    } else {
        let idx = hostport.rfind(':')?;
        let host = hostport[..idx].to_string();
        let port_str = &hostport[idx + 1..];
        (host, port_str.parse::<u16>().ok()?)
    };

    let name = name.unwrap_or_else(|| format!("{}:{}", server, port));

    let mut map = serde_yaml::Mapping::new();
    map.insert(
        serde_yaml::Value::String("name".to_string()),
        serde_yaml::Value::String(name.clone()),
    );
    map.insert(
        serde_yaml::Value::String("type".to_string()),
        serde_yaml::Value::String("ss".to_string()),
    );
    map.insert(
        serde_yaml::Value::String("server".to_string()),
        serde_yaml::Value::String(server),
    );
    map.insert(
        serde_yaml::Value::String("port".to_string()),
        serde_yaml::Value::Number(port.into()),
    );
    map.insert(
        serde_yaml::Value::String("cipher".to_string()),
        serde_yaml::Value::String(cipher),
    );
    map.insert(
        serde_yaml::Value::String("password".to_string()),
        serde_yaml::Value::String(password),
    );
    if let Some(plugin) = plugin {
        map.insert(
            serde_yaml::Value::String("plugin".to_string()),
            serde_yaml::Value::String(plugin),
        );
    }
    if let Some(opts) = plugin_opts {
        map.insert(
            serde_yaml::Value::String("plugin-opts".to_string()),
            serde_yaml::Value::String(opts),
        );
    }

    Some(ProxySpec { name, map })
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn parse_vmess_url(line: &str) -> Option<ProxySpec> {
    let content = line.trim().strip_prefix("vmess://")?;
    let decoded = decode_base64(content)?;
    let json: serde_json::Value = serde_json::from_slice(&decoded).ok()?;

    let get_str = |key: &str| {
        json.get(key).and_then(|v| match v {
            serde_json::Value::String(s) => Some(s.clone()),
            serde_json::Value::Number(n) => Some(n.to_string()),
            _ => None,
        })
    };

    let server = get_str("add")?;
    let port: u16 = get_str("port")?.parse().ok()?;
    let uuid = get_str("id")?;
    let name = get_str("ps")
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("{}:{}", server, port));
    let alter_id = get_str("aid").and_then(|v| v.parse::<u16>().ok());
    let cipher = get_str("scy")
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "auto".to_string());
    let network = get_str("net").or_else(|| get_str("network"));
    let tls = get_str("tls").unwrap_or_default();
    let sni = get_str("sni").or_else(|| get_str("host"));
    let alpn = get_str("alpn");
    let host = get_str("host");
    let path = get_str("path");

    let mut map = serde_yaml::Mapping::new();
    map.insert(
        serde_yaml::Value::String("name".to_string()),
        serde_yaml::Value::String(name.clone()),
    );
    map.insert(
        serde_yaml::Value::String("type".to_string()),
        serde_yaml::Value::String("vmess".to_string()),
    );
    map.insert(
        serde_yaml::Value::String("server".to_string()),
        serde_yaml::Value::String(server),
    );
    map.insert(
        serde_yaml::Value::String("port".to_string()),
        serde_yaml::Value::Number(port.into()),
    );
    map.insert(
        serde_yaml::Value::String("uuid".to_string()),
        serde_yaml::Value::String(uuid),
    );
    map.insert(
        serde_yaml::Value::String("cipher".to_string()),
        serde_yaml::Value::String(cipher),
    );
    if let Some(alter_id) = alter_id {
        map.insert(
            serde_yaml::Value::String("alterId".to_string()),
            serde_yaml::Value::Number(alter_id.into()),
        );
    }
    if let Some(network) = network.clone().filter(|n| !n.is_empty()) {
        map.insert(
            serde_yaml::Value::String("network".to_string()),
            serde_yaml::Value::String(network.clone()),
        );
    }
    if !tls.is_empty() && tls != "none" {
        map.insert(
            serde_yaml::Value::String("tls".to_string()),
            serde_yaml::Value::Bool(true),
        );
    }
    if let Some(sni) = sni {
        map.insert(
            serde_yaml::Value::String("servername".to_string()),
            serde_yaml::Value::String(sni),
        );
    }
    if let Some(alpn) = alpn {
        let list = alpn
            .split(',')
            .map(|s| serde_yaml::Value::String(s.trim().to_string()))
            .collect::<Vec<_>>();
        if !list.is_empty() {
            map.insert(
                serde_yaml::Value::String("alpn".to_string()),
                serde_yaml::Value::Sequence(list),
            );
        }
    }

    if network.as_deref() == Some("ws") {
        let mut ws = serde_yaml::Mapping::new();
        if let Some(path) = path {
            ws.insert(
                serde_yaml::Value::String("path".to_string()),
                serde_yaml::Value::String(path),
            );
        }
        if let Some(host) = host {
            let mut headers = serde_yaml::Mapping::new();
            headers.insert(
                serde_yaml::Value::String("Host".to_string()),
                serde_yaml::Value::String(host),
            );
            ws.insert(
                serde_yaml::Value::String("headers".to_string()),
                serde_yaml::Value::Mapping(headers),
            );
        }
        if !ws.is_empty() {
            map.insert(
                serde_yaml::Value::String("ws-opts".to_string()),
                serde_yaml::Value::Mapping(ws),
            );
        }
    } else if network.as_deref() == Some("grpc") {
        let mut grpc = serde_yaml::Mapping::new();
        if let Some(service) = path {
            grpc.insert(
                serde_yaml::Value::String("grpc-service-name".to_string()),
                serde_yaml::Value::String(service),
            );
        }
        if !grpc.is_empty() {
            map.insert(
                serde_yaml::Value::String("grpc-opts".to_string()),
                serde_yaml::Value::Mapping(grpc),
            );
        }
    }

    Some(ProxySpec { name, map })
}

fn parse_vless_url(line: &str) -> Option<ProxySpec> {
    let url = Url::parse(line).ok()?;
    if url.scheme() != "vless" {
        return None;
    }
    let uuid = url.username().to_string();
    if uuid.is_empty() {
        return None;
    }
    let server = url.host_str()?.to_string();
    let port = url.port()?;
    let name = url
        .fragment()
        .map(percent_decode)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("{}:{}", server, port));

    let mut params = std::collections::HashMap::new();
    for (key, value) in url::form_urlencoded::parse(url.query().unwrap_or("").as_bytes()) {
        params.insert(key.to_string(), value.to_string());
    }

    let network = params
        .get("type")
        .cloned()
        .or_else(|| params.get("network").cloned());
    let security = params
        .get("security")
        .cloned()
        .unwrap_or_else(|| "none".to_string());
    let sni = params
        .get("sni")
        .cloned()
        .or_else(|| params.get("peer").cloned());
    let alpn = params.get("alpn").cloned();
    let flow = params.get("flow").cloned();
    let encryption = params.get("encryption").cloned();
    let udp = params
        .get("udp")
        .and_then(|v| parse_bool(v))
        .unwrap_or(false);

    let mut map = serde_yaml::Mapping::new();
    map.insert(
        serde_yaml::Value::String("name".to_string()),
        serde_yaml::Value::String(name.clone()),
    );
    map.insert(
        serde_yaml::Value::String("type".to_string()),
        serde_yaml::Value::String("vless".to_string()),
    );
    map.insert(
        serde_yaml::Value::String("server".to_string()),
        serde_yaml::Value::String(server),
    );
    map.insert(
        serde_yaml::Value::String("port".to_string()),
        serde_yaml::Value::Number((port as u16).into()),
    );
    map.insert(
        serde_yaml::Value::String("uuid".to_string()),
        serde_yaml::Value::String(uuid),
    );
    map.insert(
        serde_yaml::Value::String("udp".to_string()),
        serde_yaml::Value::Bool(udp),
    );
    if let Some(network) = network.clone().filter(|n| !n.is_empty()) {
        map.insert(
            serde_yaml::Value::String("network".to_string()),
            serde_yaml::Value::String(network.clone()),
        );
    }
    if let Some(flow) = flow {
        map.insert(
            serde_yaml::Value::String("flow".to_string()),
            serde_yaml::Value::String(flow),
        );
    }
    if let Some(encryption) = encryption {
        map.insert(
            serde_yaml::Value::String("encryption".to_string()),
            serde_yaml::Value::String(encryption),
        );
    }
    if security != "none" {
        map.insert(
            serde_yaml::Value::String("tls".to_string()),
            serde_yaml::Value::Bool(true),
        );
    }
    if let Some(sni) = sni {
        map.insert(
            serde_yaml::Value::String("servername".to_string()),
            serde_yaml::Value::String(sni),
        );
    }
    if let Some(alpn) = alpn {
        let list = alpn
            .split(',')
            .map(|s| serde_yaml::Value::String(s.trim().to_string()))
            .collect::<Vec<_>>();
        if !list.is_empty() {
            map.insert(
                serde_yaml::Value::String("alpn".to_string()),
                serde_yaml::Value::Sequence(list),
            );
        }
    }

    if security == "reality" {
        let mut reality = serde_yaml::Mapping::new();
        if let Some(pbk) = params
            .get("pbk")
            .cloned()
            .or_else(|| params.get("public-key").cloned())
        {
            reality.insert(
                serde_yaml::Value::String("public-key".to_string()),
                serde_yaml::Value::String(pbk),
            );
        }
        if let Some(sid) = params
            .get("sid")
            .cloned()
            .or_else(|| params.get("short-id").cloned())
        {
            reality.insert(
                serde_yaml::Value::String("short-id".to_string()),
                serde_yaml::Value::String(sid),
            );
        }
        if let Some(spx) = params
            .get("spx")
            .cloned()
            .or_else(|| params.get("spider-x").cloned())
        {
            reality.insert(
                serde_yaml::Value::String("spider-x".to_string()),
                serde_yaml::Value::String(spx),
            );
        }
        if let Some(fp) = params.get("fp").cloned() {
            reality.insert(
                serde_yaml::Value::String("fingerprint".to_string()),
                serde_yaml::Value::String(fp),
            );
        }
        if !reality.is_empty() {
            map.insert(
                serde_yaml::Value::String("reality-opts".to_string()),
                serde_yaml::Value::Mapping(reality),
            );
        }
    }

    if network.as_deref() == Some("ws") {
        let mut ws = serde_yaml::Mapping::new();
        if let Some(path) = params.get("path") {
            ws.insert(
                serde_yaml::Value::String("path".to_string()),
                serde_yaml::Value::String(path.clone()),
            );
        }
        if let Some(host) = params.get("host") {
            let mut headers = serde_yaml::Mapping::new();
            headers.insert(
                serde_yaml::Value::String("Host".to_string()),
                serde_yaml::Value::String(host.clone()),
            );
            ws.insert(
                serde_yaml::Value::String("headers".to_string()),
                serde_yaml::Value::Mapping(headers),
            );
        }
        if !ws.is_empty() {
            map.insert(
                serde_yaml::Value::String("ws-opts".to_string()),
                serde_yaml::Value::Mapping(ws),
            );
        }
    } else if network.as_deref() == Some("grpc") {
        let mut grpc = serde_yaml::Mapping::new();
        let service_name = params
            .get("serviceName")
            .cloned()
            .or_else(|| params.get("service").cloned())
            .or_else(|| params.get("path").cloned());
        if let Some(service) = service_name {
            grpc.insert(
                serde_yaml::Value::String("grpc-service-name".to_string()),
                serde_yaml::Value::String(service),
            );
        }
        if !grpc.is_empty() {
            map.insert(
                serde_yaml::Value::String("grpc-opts".to_string()),
                serde_yaml::Value::Mapping(grpc),
            );
        }
    }

    Some(ProxySpec { name, map })
}

fn parse_trojan_url(line: &str) -> Option<ProxySpec> {
    let url = Url::parse(line).ok()?;
    if url.scheme() != "trojan" {
        return None;
    }
    let password = url.username().to_string();
    if password.is_empty() {
        return None;
    }
    let server = url.host_str()?.to_string();
    let port = url.port()?;
    let name = url
        .fragment()
        .map(percent_decode)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("{}:{}", server, port));

    let mut params = std::collections::HashMap::new();
    for (key, value) in url::form_urlencoded::parse(url.query().unwrap_or("").as_bytes()) {
        params.insert(key.to_string(), value.to_string());
    }

    let network = params
        .get("type")
        .cloned()
        .or_else(|| params.get("network").cloned());
    let sni = params
        .get("sni")
        .cloned()
        .or_else(|| params.get("peer").cloned());
    let alpn = params.get("alpn").cloned();
    let udp = params
        .get("udp")
        .and_then(|v| parse_bool(v))
        .unwrap_or(false);
    let skip_cert = params
        .get("allowInsecure")
        .or_else(|| params.get("skip-cert-verify"))
        .and_then(|v| parse_bool(v))
        .unwrap_or(false);

    let mut map = serde_yaml::Mapping::new();
    map.insert(
        serde_yaml::Value::String("name".to_string()),
        serde_yaml::Value::String(name.clone()),
    );
    map.insert(
        serde_yaml::Value::String("type".to_string()),
        serde_yaml::Value::String("trojan".to_string()),
    );
    map.insert(
        serde_yaml::Value::String("server".to_string()),
        serde_yaml::Value::String(server),
    );
    map.insert(
        serde_yaml::Value::String("port".to_string()),
        serde_yaml::Value::Number((port as u16).into()),
    );
    map.insert(
        serde_yaml::Value::String("password".to_string()),
        serde_yaml::Value::String(password),
    );
    map.insert(
        serde_yaml::Value::String("udp".to_string()),
        serde_yaml::Value::Bool(udp),
    );
    if skip_cert {
        map.insert(
            serde_yaml::Value::String("skip-cert-verify".to_string()),
            serde_yaml::Value::Bool(true),
        );
    }
    if let Some(network) = network.clone().filter(|n| !n.is_empty()) {
        map.insert(
            serde_yaml::Value::String("network".to_string()),
            serde_yaml::Value::String(network.clone()),
        );
    }
    if let Some(sni) = sni {
        map.insert(
            serde_yaml::Value::String("sni".to_string()),
            serde_yaml::Value::String(sni),
        );
    }
    if let Some(alpn) = alpn {
        let list = alpn
            .split(',')
            .map(|s| serde_yaml::Value::String(s.trim().to_string()))
            .collect::<Vec<_>>();
        if !list.is_empty() {
            map.insert(
                serde_yaml::Value::String("alpn".to_string()),
                serde_yaml::Value::Sequence(list),
            );
        }
    }

    if network.as_deref() == Some("ws") {
        let mut ws = serde_yaml::Mapping::new();
        if let Some(path) = params.get("path") {
            ws.insert(
                serde_yaml::Value::String("path".to_string()),
                serde_yaml::Value::String(path.clone()),
            );
        }
        if let Some(host) = params.get("host") {
            let mut headers = serde_yaml::Mapping::new();
            headers.insert(
                serde_yaml::Value::String("Host".to_string()),
                serde_yaml::Value::String(host.clone()),
            );
            ws.insert(
                serde_yaml::Value::String("headers".to_string()),
                serde_yaml::Value::Mapping(headers),
            );
        }
        if !ws.is_empty() {
            map.insert(
                serde_yaml::Value::String("ws-opts".to_string()),
                serde_yaml::Value::Mapping(ws),
            );
        }
    } else if network.as_deref() == Some("grpc") {
        let mut grpc = serde_yaml::Mapping::new();
        if let Some(service) = params.get("serviceName") {
            grpc.insert(
                serde_yaml::Value::String("grpc-service-name".to_string()),
                serde_yaml::Value::String(service.clone()),
            );
        }
        if !grpc.is_empty() {
            map.insert(
                serde_yaml::Value::String("grpc-opts".to_string()),
                serde_yaml::Value::Mapping(grpc),
            );
        }
    }

    Some(ProxySpec { name, map })
}

fn parse_raw_subscription(bytes: &[u8]) -> Vec<ProxySpec> {
    let mut proxies = Vec::new();
    for line in extract_subscription_lines(bytes) {
        if let Some(proxy) = parse_ss_url(&line) {
            proxies.push(proxy);
            continue;
        }
        if let Some(proxy) = parse_vmess_url(&line) {
            proxies.push(proxy);
            continue;
        }
        if let Some(proxy) = parse_vless_url(&line) {
            proxies.push(proxy);
            continue;
        }
        if let Some(proxy) = parse_trojan_url(&line) {
            proxies.push(proxy);
        }
    }
    proxies
}

fn convert_raw_subscription_to_config(
    raw_bytes: &[u8],
    base_config_path: &Path,
) -> Result<(Vec<u8>, usize), String> {
    let proxies = parse_raw_subscription(raw_bytes);
    if proxies.is_empty() {
        return Err("Unsupported raw subscription format".to_string());
    }
    let base_bytes = std::fs::read(base_config_path)
        .map_err(|e| format!("Failed to read base config: {}", e))?;
    let output = apply_proxies_to_config(&base_bytes, &proxies)?;
    Ok((output, proxies.len()))
}

fn proxy_specs_to_yaml(proxies: &[ProxySpec]) -> serde_yaml::Value {
    let mut items = Vec::new();
    for proxy in proxies {
        items.push(serde_yaml::Value::Mapping(proxy.map.clone()));
    }
    serde_yaml::Value::Sequence(items)
}

fn apply_proxies_to_config(base_bytes: &[u8], proxies: &[ProxySpec]) -> Result<Vec<u8>, String> {
    let mut config_value: serde_yaml::Value = serde_yaml::from_slice(base_bytes)
        .unwrap_or_else(|_| serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));

    let config_map = match config_value.as_mapping_mut() {
        Some(map) => map,
        None => {
            config_value = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
            config_value.as_mapping_mut().unwrap()
        }
    };

    config_map.insert(
        serde_yaml::Value::String("proxies".to_string()),
        proxy_specs_to_yaml(proxies),
    );

    let proxy_names: Vec<String> = proxies.iter().map(|p| p.name.clone()).collect();
    let mut group_names = Vec::new();

    if let Some(serde_yaml::Value::Sequence(groups)) =
        config_map.get(&serde_yaml::Value::String("proxy-groups".to_string()))
    {
        for group in groups {
            if let Some(name) = group
                .as_mapping()
                .and_then(|map| map.get(&serde_yaml::Value::String("name".to_string())))
                .and_then(|v| v.as_str())
            {
                group_names.push(name.to_string());
            }
        }
    }

    let special = ["DIRECT", "REJECT", "REJECT-DROP", "PASS", "GLOBAL"];

    if let Some(serde_yaml::Value::Sequence(groups)) =
        config_map.get_mut(&serde_yaml::Value::String("proxy-groups".to_string()))
    {
        for group in groups {
            let group_map = match group.as_mapping_mut() {
                Some(map) => map,
                None => continue,
            };
            let proxies_value =
                match group_map.get(&serde_yaml::Value::String("proxies".to_string())) {
                    Some(serde_yaml::Value::Sequence(list)) => list.clone(),
                    _ => continue,
                };

            let mut has_proxy_entries = false;
            for entry in &proxies_value {
                if let Some(name) = entry.as_str() {
                    let is_group = group_names.iter().any(|g| g == name);
                    let is_special = special.iter().any(|s| s == &name);
                    if !is_group && !is_special {
                        has_proxy_entries = true;
                        break;
                    }
                }
            }

            if !has_proxy_entries {
                continue;
            }

            let mut new_list = Vec::new();
            let mut seen = std::collections::HashSet::new();

            for entry in proxies_value {
                if let Some(name) = entry.as_str() {
                    let is_group = group_names.iter().any(|g| g == name);
                    let is_special = special.iter().any(|s| s == &name);
                    if is_group || is_special {
                        if seen.insert(name.to_string()) {
                            new_list.push(serde_yaml::Value::String(name.to_string()));
                        }
                    }
                }
            }

            for name in &proxy_names {
                if seen.insert(name.clone()) {
                    new_list.push(serde_yaml::Value::String(name.clone()));
                }
            }

            group_map.insert(
                serde_yaml::Value::String("proxies".to_string()),
                serde_yaml::Value::Sequence(new_list),
            );
        }
    }

    serde_yaml::to_string(&config_value)
        .map(|s| s.into_bytes())
        .map_err(|e| format!("Failed to serialize config: {}", e))
}

pub async fn run(
    api_url: String,
    secret: Option<String>,
    preset: Preset,
    config: &mut AppConfig,
) -> Result<()> {
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
    let mut routes_expanded = false; // Whether viewing node list
    let mut selected_node_index = 0;
    let mut show_quit_confirmation = false; // Whether showing quit confirmation dialog
    let mut rules_search_query = String::new(); // Search query for rules
    let mut rules_search_mode = false; // Whether in search mode
    let mut rules_edit_mode = pages::RuleEditMode::None; // Rule edit mode
    let mut rules_edit_input = String::new(); // Rule edit input
    let mut rules_selected_index = 0; // Selected rule index in Simple mode
    let mut rules_list_focus = pages::RuleListFocus::Whitelist; // Which list is focused in Simple mode
    let mut connections_data: Option<ConnectionsResponse> = None; // Connections data
    let mut connections_selected_index = 0; // Selected connection index
    let mut connections_scroll_offset = 0; // Connections scroll offset
    let mut connections_last_refresh = std::time::Instant::now();
    let mut connections_search_query = String::new(); // Connections search query
    let mut connections_search_mode = false; // Connections search mode
    let mut settings_action = pages::SettingsAction::None; // Settings page action state
    let mut logs_data: Vec<crate::clash::LogEntry> = Vec::new(); // Logs data
    let mut logs_level_filter = pages::LogLevel::All; // Logs level filter
    let mut logs_search_query = String::new(); // Logs search query
    let mut logs_search_mode = false; // Logs search mode
    let mut logs_scroll_offset = 0; // Logs scroll offset
    let (logs_tx, mut logs_rx) = mpsc::unbounded_channel::<LogStreamEvent>();
    let mut logs_task: Option<JoinHandle<()>> = None;
    let mut logs_shutdown: Option<watch::Sender<bool>> = None;
    let mut logs_connected = false;
    let mut logs_status_detail: Option<String> = None;
    let mut performance_last_refresh = std::time::Instant::now();
    let mut performance_upload_total = 0u64;
    let mut performance_download_total = 0u64;
    let mut performance_upload_rate = 0u64;
    let mut performance_download_rate = 0u64;
    let mut performance_connection_count = 0usize;
    let mut update_providers: Vec<SubscriptionItem> = Vec::new();
    let mut update_selected_index = 0;
    let mut _update_last_refresh = std::time::Instant::now();
    let mut rules_data: Vec<crate::clash::Rule> = Vec::new(); // Rules data from API
    let (update_tx, mut update_rx) = mpsc::unbounded_channel::<UpdateEvent>();
    let mut update_in_flight = 0usize;
    let mut update_total = 0usize;
    let mut update_success = 0usize;
    let mut update_fail = 0usize;

    loop {
        // Process any pending delay test results
        state.process_delay_results();

        while let Ok(event) = logs_rx.try_recv() {
            match event {
                LogStreamEvent::Entry(entry) => {
                    logs_data.insert(0, entry);
                    if logs_data.len() > 1000 {
                        logs_data.truncate(1000);
                    }
                }
                LogStreamEvent::Status(status) => match status {
                    LogStreamStatus::Connected => {
                        logs_connected = true;
                        logs_status_detail = None;
                    }
                    LogStreamStatus::Disconnected(reason) => {
                        logs_connected = false;
                        logs_status_detail = Some(reason);
                    }
                },
            }
        }

        while let Ok(event) = update_rx.try_recv() {
            match event {
                UpdateEvent::ItemFinished {
                    index,
                    name,
                    updated_at,
                    success,
                    error,
                } => {
                    if let Some(updated_at) = updated_at {
                        if index < update_providers.len() {
                            update_providers[index].updated_at = Some(updated_at);
                        }
                    }

                    if update_in_flight > 0 {
                        update_in_flight -= 1;
                    }

                    if success {
                        update_success += 1;
                    } else {
                        update_fail += 1;
                    }

                    let completed = update_success + update_fail;
                    if update_in_flight == 0 && update_total > 0 {
                        if update_total == 1 {
                            if success {
                                state.status_message =
                                    Some(format!("Updated {} successfully!", name));
                            } else {
                                let detail = error.unwrap_or_else(|| "Unknown error".to_string());
                                state.status_message =
                                    Some(format!("Failed to update {}: {}", name, detail));
                            }
                        } else if update_fail == 0 {
                            state.status_message = Some(format!(
                                "All {} providers updated successfully!",
                                update_success
                            ));
                        } else {
                            state.status_message = Some(format!(
                                "Updated: {} succeeded, {} failed",
                                update_success, update_fail
                            ));
                        }
                    } else if update_total > 0 {
                        state.status_message =
                            Some(format!("Updating... ({}/{})", completed, update_total));
                    }

                    if update_in_flight == 0 && update_total > 0 {
                        refresh_update_providers(state, config, &mut update_providers).await;
                        update_selected_index =
                            update_selected_index.min(update_providers.len().saturating_sub(1));
                        update_total = 0;
                    }
                }
            }
        }

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
                    Err(e) => {
                        state.status_message = Some(format!("Failed to fetch connections: {}", e))
                    }
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
                            performance_upload_rate =
                                (data.upload_total.saturating_sub(performance_upload_total))
                                    / elapsed_secs;
                            performance_download_rate = (data
                                .download_total
                                .saturating_sub(performance_download_total))
                                / elapsed_secs;
                        }
                        performance_upload_total = data.upload_total;
                        performance_download_total = data.download_total;
                        performance_connection_count = data.connections.len();
                    }
                    Err(e) => {
                        state.status_message =
                            Some(format!("Failed to fetch performance data: {}", e))
                    }
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
                        pages::render_routes_with_nodes(
                            f,
                            chunks[1],
                            state,
                            config,
                            selected_route_index,
                            selected_node_index,
                        )
                    } else {
                        pages::render_routes(f, chunks[1], state, config, selected_route_index)
                    }
                }
                Page::Rules => pages::render_rules(
                    f,
                    chunks[1],
                    state,
                    rules_scroll_offset,
                    &rules_search_query,
                    rules_search_mode,
                    rules_edit_mode,
                    &rules_edit_input,
                    config,
                    rules_selected_index,
                    &rules_data,
                    rules_list_focus,
                ),
                Page::Update => pages::render_update(
                    f,
                    chunks[1],
                    state,
                    &update_providers,
                    update_selected_index,
                ),
                Page::Connections => pages::render_connections(
                    f,
                    chunks[1],
                    state,
                    connections_data.as_ref(),
                    connections_selected_index,
                    connections_scroll_offset,
                    &connections_search_query,
                    connections_search_mode,
                ),
                Page::Settings => {
                    pages::render_settings(f, chunks[1], state, config, &settings_action)
                }
                Page::Logs => pages::render_logs(
                    f,
                    chunks[1],
                    state,
                    &logs_data,
                    logs_level_filter,
                    &logs_search_query,
                    logs_scroll_offset,
                    logs_connected,
                    logs_status_detail.as_deref(),
                ),
                Page::Performance => pages::render_performance(
                    f,
                    chunks[1],
                    state,
                    performance_upload_total,
                    performance_download_total,
                    performance_upload_rate,
                    performance_download_rate,
                    performance_connection_count,
                ),
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
                                Err(e) => {
                                    state.status_message =
                                        Some(format!("Failed to fetch connections: {}", e))
                                }
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
                            state.status_message =
                                Some(format!("Switched to {} theme", next_theme.name()));
                        }
                        // Note: 't' key for speed test is removed from Home page
                        KeyCode::Char('m') => {
                            // Switch to next mode (Rule -> Global -> Direct -> Rule)
                            let next_mode = state.clash_state.mode.next();
                            if let Err(e) = state.switch_mode(next_mode).await {
                                state.status_message =
                                    Some(format!("Failed to switch mode: {}", e));
                            }
                            last_refresh = std::time::Instant::now();
                        }
                        KeyCode::Char('g') => {
                            state.current_page = Page::Routes;
                            selected_route_index = 0;
                            selected_node_index = 0;
                            routes_expanded = false;
                            let _ = state.refresh().await;
                            last_refresh = std::time::Instant::now();
                        }
                        KeyCode::Char('l') => {
                            state.current_page = Page::Rules;
                            rules_scroll_offset = 0;
                            // Fetch rules immediately
                            match state.clash_state.client.get_rules().await {
                                Ok(rules_response) => rules_data = rules_response.rules,
                                Err(e) => {
                                    state.status_message =
                                        Some(format!("Failed to fetch rules: {}", e))
                                }
                            }
                        }
                        KeyCode::Char('u') => {
                            state.current_page = Page::Update;
                            update_selected_index = 0;
                            refresh_update_providers(state, config, &mut update_providers).await;
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
                            state.status_message = Some(format!(
                                "Switched to {} preset: {}",
                                state.preset.name(),
                                state.preset.description()
                            ));
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
                                Err(e) => {
                                    state.status_message =
                                        Some(format!("Failed to fetch performance data: {}", e))
                                }
                            }
                            performance_last_refresh = std::time::Instant::now();
                        }
                        KeyCode::Char('o') => {
                            state.current_page = Page::Logs;
                            logs_scroll_offset = 0;
                            logs_search_mode = false;
                            logs_search_query.clear();
                            logs_data.clear();
                            logs_connected = false;
                            logs_status_detail = Some("connecting".to_string());
                            start_logs_stream(
                                state.clash_state.client.clone(),
                                log_level_to_ws(logs_level_filter),
                                logs_tx.clone(),
                                &mut logs_shutdown,
                                &mut logs_task,
                            );
                        }
                        _ => {}
                    },
                    Page::Routes => {
                        let routes = crate::clash::HumanRoute::from_proxies(
                            &state.clash_state.proxies,
                            state.mode,
                        );

                        if !routes_expanded {
                            // Route list mode
                            let max_index = routes.len().saturating_sub(1);

                            match key.code {
                                KeyCode::Char('q') | KeyCode::Esc => {
                                    // Return to Home instead of quitting
                                    state.current_page = Page::Home;
                                }
                                KeyCode::Char('h') => state.current_page = Page::Home,
                                KeyCode::Char('r') => {
                                    state.status_message = Some("Refreshing routes...".to_string());
                                    match state.refresh().await {
                                        Ok(()) => {
                                            routes_expanded = false;
                                            selected_route_index = 0;
                                            selected_node_index = 0;
                                            state.status_message =
                                                Some("Routes refreshed".to_string());
                                        }
                                        Err(e) => {
                                            state.status_message =
                                                Some(format!("Refresh failed: {}", e));
                                        }
                                    }
                                }
                                KeyCode::Char('p')
                                    if key.modifiers.contains(KeyModifiers::CONTROL) =>
                                {
                                    // Cycle to next preset
                                    state.preset = state.preset.next();
                                    state.status_message = Some(format!(
                                        "Switched to {} preset: {}",
                                        state.preset.name(),
                                        state.preset.description()
                                    ));
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
                                            if let Some(idx) =
                                                route.all_nodes.iter().position(|n| n == current)
                                            {
                                                selected_node_index = idx;
                                            }
                                        }
                                    }
                                }
                                KeyCode::Char('t') | KeyCode::Char('T') => {
                                    // Batch test all nodes in selected route (only if preset allows)
                                    if state.preset.show_speed_test()
                                        && selected_route_index < routes.len()
                                    {
                                        let route = &routes[selected_route_index];
                                        // Filter out non-testable nodes (Direct, Reject, etc.) silently
                                        let testable_nodes: Vec<String> = route
                                            .all_nodes
                                            .iter()
                                            .filter(|node| state.is_node_testable(node))
                                            .cloned()
                                            .collect();

                                        if !testable_nodes.is_empty() {
                                            state.status_message = Some(format!(
                                                "Testing {} nodes in {}...",
                                                testable_nodes.len(),
                                                route.display_name()
                                            ));
                                            for node in testable_nodes {
                                                state.start_test_delay(node);
                                            }
                                        }
                                        // Silently skip if no testable nodes
                                    } else if !state.preset.show_speed_test() {
                                        state.status_message = Some(
                                            "Speed test disabled in current preset".to_string(),
                                        );
                                    }
                                }
                                KeyCode::Char('c')
                                    if key.modifiers.contains(KeyModifiers::CONTROL) =>
                                {
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
                                            state.status_message =
                                                Some(format!("Failed to switch: {}", e));
                                        }

                                        last_refresh = std::time::Instant::now();
                                        // Stay in node selection mode to see the change
                                    }
                                }
                                KeyCode::Char('t') | KeyCode::Char('T') => {
                                    // Batch test all nodes in this route (only if preset allows)
                                    if state.preset.show_speed_test() {
                                        // Filter out non-testable nodes (Direct, Reject, etc.) silently
                                        let testable_nodes: Vec<String> = route
                                            .all_nodes
                                            .iter()
                                            .filter(|node| state.is_node_testable(node))
                                            .cloned()
                                            .collect();

                                        if !testable_nodes.is_empty() {
                                            state.status_message = Some(format!(
                                                "Testing {} nodes...",
                                                testable_nodes.len()
                                            ));
                                            for node in testable_nodes {
                                                state.start_test_delay(node);
                                            }
                                        }
                                        // Silently skip if no testable nodes
                                    } else {
                                        state.status_message = Some(
                                            "Speed test disabled in current preset".to_string(),
                                        );
                                    }
                                }
                                KeyCode::Char('*') => {
                                    // Toggle favorite for selected node
                                    if selected_node_index < route.all_nodes.len() {
                                        let node = &route.all_nodes[selected_node_index];
                                        if config.is_favorite(node) {
                                            if let Err(e) = config.remove_favorite(node) {
                                                state.status_message = Some(format!(
                                                    "Failed to remove favorite: {}",
                                                    e
                                                ));
                                            } else {
                                                state.status_message = Some(format!(
                                                    "Removed {} from favorites",
                                                    node
                                                ));
                                            }
                                        } else {
                                            if let Err(e) = config.add_favorite(node.clone()) {
                                                state.status_message =
                                                    Some(format!("Failed to add favorite: {}", e));
                                            } else {
                                                state.status_message =
                                                    Some(format!("Added {} to favorites", node));
                                            }
                                        }
                                    }
                                }
                                KeyCode::Char('c')
                                    if key.modifiers.contains(KeyModifiers::CONTROL) =>
                                {
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
                                        let result =
                                            match rules_edit_mode {
                                                pages::RuleEditMode::AddWhitelist => config
                                                    .add_to_whitelist(rules_edit_input.clone()),
                                                pages::RuleEditMode::AddBlacklist => config
                                                    .add_to_blacklist(rules_edit_input.clone()),
                                                pages::RuleEditMode::None => Ok(()),
                                            };

                                        if let Err(e) = result {
                                            state.status_message =
                                                Some(format!("Failed to save rule: {}", e));
                                        } else {
                                            state.status_message =
                                                Some(format!("Rule added: {}", rules_edit_input));
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
                                            state.status_message =
                                                Some(format!("Loaded {} rules", rules_data.len()));
                                        }
                                        Err(e) => {
                                            state.status_message =
                                                Some(format!("Failed to refresh: {}", e))
                                        }
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
                                                let domain =
                                                    config.whitelist[rules_selected_index].clone();
                                                config.remove_from_whitelist(&domain)
                                            } else {
                                                Ok(())
                                            }
                                        }
                                        pages::RuleListFocus::Blacklist => {
                                            if rules_selected_index < config.blacklist.len() {
                                                let domain =
                                                    config.blacklist[rules_selected_index].clone();
                                                config.remove_from_blacklist(&domain)
                                            } else {
                                                Ok(())
                                            }
                                        }
                                    };

                                    if let Err(e) = result {
                                        state.status_message =
                                            Some(format!("Failed to delete rule: {}", e));
                                    } else {
                                        state.status_message = Some("Rule deleted".to_string());
                                        // Adjust selected index if needed
                                        let list_len = match rules_list_focus {
                                            pages::RuleListFocus::Whitelist => {
                                                config.whitelist.len()
                                            }
                                            pages::RuleListFocus::Blacklist => {
                                                config.blacklist.len()
                                            }
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
                                KeyCode::Char('p')
                                    if key.modifiers.contains(KeyModifiers::CONTROL) =>
                                {
                                    state.preset = state.preset.next();
                                    let _ = config.set_preset(&state.preset);
                                    state.status_message = Some(format!(
                                        "Switched to {} preset: {}",
                                        state.preset.name(),
                                        state.preset.description()
                                    ));
                                }
                                KeyCode::Char('c')
                                    if key.modifiers.contains(KeyModifiers::CONTROL) =>
                                {
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
                                if update_in_flight > 0 {
                                    state.status_message =
                                        Some("Update in progress...".to_string());
                                } else {
                                    state.status_message =
                                        Some("Refreshing providers...".to_string());
                                    refresh_update_providers(state, config, &mut update_providers)
                                        .await;
                                    if state.status_message.as_deref()
                                        == Some("Refreshing providers...")
                                    {
                                        state.status_message =
                                            Some("Providers refreshed!".to_string());
                                    }
                                    _update_last_refresh = std::time::Instant::now();
                                }
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
                                if update_in_flight > 0 {
                                    state.status_message =
                                        Some("Update in progress...".to_string());
                                } else if update_selected_index < update_providers.len() {
                                    let item = update_providers[update_selected_index].clone();
                                    update_total = 1;
                                    update_in_flight = 1;
                                    update_success = 0;
                                    update_fail = 0;
                                    state.status_message =
                                        Some(format!("Updating {}...", item.name));
                                    spawn_update_task(
                                        update_tx.clone(),
                                        item,
                                        update_selected_index,
                                        state.clash_state.client.clone(),
                                    );
                                } else {
                                    state.status_message =
                                        Some("No subscriptions to update".to_string());
                                }
                            }
                            KeyCode::Char('s') => {
                                // Switch current subscription (Mihomo Party)
                                if update_selected_index < update_providers.len() {
                                    let item = update_providers[update_selected_index].clone();
                                    debug_log(&format!(
                                        "switch start name='{}' type='{}' url_present={}",
                                        item.name,
                                        item.provider_type,
                                        item.url.is_some()
                                    ));
                                    match &item.source {
                                        SubscriptionSource::MihomoPartyProfile {
                                            id,
                                            profile_path,
                                            list_path,
                                        } => {
                                            debug_log(&format!(
                                                "switch profile id={} path={} list={}",
                                                id,
                                                profile_path.display(),
                                                list_path.display()
                                            ));
                                            let work_config_path =
                                                mihomo_party::work_config_path_from_list(list_path)
                                                    .unwrap_or_else(|| {
                                                        list_path
                                                            .parent()
                                                            .unwrap_or_else(|| Path::new("."))
                                                            .join("work")
                                                            .join("config.yaml")
                                                    });
                                            if !profile_path.is_file() {
                                                if let Some(url) = item.url.as_deref() {
                                                    if is_http_url(url) {
                                                        if let Err(e) = update_mihomo_party_profile(
                                                            id,
                                                            url,
                                                            profile_path,
                                                            list_path,
                                                        )
                                                        .await
                                                        {
                                                            state.status_message = Some(format!(
                                                                "Failed to download subscription: {}",
                                                                e
                                                            ));
                                                            debug_log(&format!(
                                                                "switch update_profile failed: {}",
                                                                e
                                                            ));
                                                            continue;
                                                        }
                                                    } else {
                                                        let bytes = match std::fs::read(url) {
                                                            Ok(bytes) => bytes,
                                                            Err(e) => {
                                                                state.status_message = Some(
                                                                    format!(
                                                                        "Failed to read subscription file: {}",
                                                                        e
                                                                    ),
                                                                );
                                                                debug_log(&format!(
                                                                    "switch read file failed: {}",
                                                                    e
                                                                ));
                                                                continue;
                                                            }
                                                        };
                                                        if let Some(parent) = profile_path.parent()
                                                        {
                                                            let _ = std::fs::create_dir_all(parent);
                                                        }
                                                        if let Err(e) =
                                                            std::fs::write(profile_path, &bytes)
                                                        {
                                                            state.status_message = Some(format!(
                                                                "Failed to write profile: {}",
                                                                e
                                                            ));
                                                            debug_log(&format!(
                                                                "switch write profile failed: {}",
                                                                e
                                                            ));
                                                            continue;
                                                        }
                                                        let updated_at =
                                                            Utc::now().timestamp_millis();
                                                        let _ =
                                                            mihomo_party::update_profile_updated_at(
                                                                list_path, id, updated_at,
                                                            );
                                                    }
                                                } else {
                                                    state.status_message = Some(
                                                        "Profile file not found, please update first"
                                                            .to_string(),
                                                    );
                                                    debug_log("switch profile missing");
                                                    continue;
                                                }
                                            }

                                            let bytes = match std::fs::read(profile_path) {
                                                Ok(bytes) => bytes,
                                                Err(e) => {
                                                    state.status_message = Some(format!(
                                                        "Failed to read profile: {}",
                                                        e
                                                    ));
                                                    debug_log(&format!(
                                                        "switch read profile failed: {}",
                                                        e
                                                    ));
                                                    continue;
                                                }
                                            };

                                            let mut applied_proxy_count = None;
                                            let output_bytes = if looks_like_clash_config(&bytes) {
                                                debug_log(&format!(
                                                    "switch profile looks_like_config bytes={}",
                                                    bytes.len()
                                                ));
                                                bytes
                                            } else {
                                                debug_log(&format!(
                                                    "switch profile raw bytes={}",
                                                    bytes.len()
                                                ));
                                                match convert_raw_subscription_to_config(
                                                    &bytes,
                                                    &work_config_path,
                                                ) {
                                                    Ok((output, count)) => {
                                                        applied_proxy_count = Some(count);
                                                        debug_log(&format!(
                                                            "switch raw converted count={} output_bytes={}",
                                                            count,
                                                            output.len()
                                                        ));
                                                        output
                                                    }
                                                    Err(e) => {
                                                        state.status_message = Some(e);
                                                        debug_log("switch raw convert failed");
                                                        continue;
                                                    }
                                                }
                                            };

                                            if applied_proxy_count.is_some() {
                                                let _ = std::fs::write(profile_path, &output_bytes);
                                            }

                                            if let Some(parent) = work_config_path.parent() {
                                                let _ = std::fs::create_dir_all(parent);
                                            }
                                            if let Err(e) =
                                                std::fs::write(&work_config_path, &output_bytes)
                                            {
                                                state.status_message = Some(format!(
                                                    "Failed to apply subscription: {}",
                                                    e
                                                ));
                                                debug_log(&format!(
                                                    "switch write work config failed: {}",
                                                    e
                                                ));
                                                continue;
                                            }

                                            let path_str =
                                                work_config_path.to_string_lossy().to_string();
                                            let temp_path = work_config_path
                                                .with_file_name("config.switch.yaml");
                                            let temp_path_str =
                                                temp_path.to_string_lossy().to_string();

                                            let mut reload_result: Option<
                                                Result<(), anyhow::Error>,
                                            > = None;
                                            if std::fs::write(&temp_path, &output_bytes).is_ok() {
                                                if state
                                                    .clash_state
                                                    .client
                                                    .reload_config_path(&temp_path_str)
                                                    .await
                                                    .is_ok()
                                                {
                                                    debug_log("switch temp path reload ok");
                                                    reload_result = Some(
                                                        state
                                                            .clash_state
                                                            .client
                                                            .reload_config_path(&path_str)
                                                            .await,
                                                    );
                                                }
                                                let _ = std::fs::remove_file(&temp_path);
                                            }

                                            let reload_result = match reload_result {
                                                Some(result) => result,
                                                None => {
                                                    state
                                                        .clash_state
                                                        .client
                                                        .reload_config_path(&path_str)
                                                        .await
                                                }
                                            };

                                            match reload_result {
                                                Ok(()) => {
                                                    debug_log("switch reload ok");
                                                    let _ = mihomo_party::set_current_profile(
                                                        list_path, id,
                                                    );
                                                    for provider in update_providers.iter_mut() {
                                                        provider.is_current = matches!(
                                                            &provider.source,
                                                            SubscriptionSource::MihomoPartyProfile { id: pid, .. }
                                                                if pid == id
                                                        );
                                                    }

                                                    let _ = state.refresh().await;
                                                    match state.clash_state.client.get_rules().await
                                                    {
                                                        Ok(rules_response) => {
                                                            rules_data = rules_response.rules;
                                                            debug_log(&format!(
                                                                "switch rules_count={}",
                                                                rules_data.len()
                                                            ));
                                                        }
                                                        Err(e) => {
                                                            debug_log(&format!(
                                                                "switch rules fetch failed: {}",
                                                                e
                                                            ));
                                                        }
                                                    }
                                                    if let Some(group) =
                                                        state.clash_state.proxies.get("🔰 节点选择")
                                                    {
                                                        if let Some(all) = &group.all {
                                                            debug_log(&format!(
                                                                "switch refresh group_nodes={}",
                                                                all.len()
                                                            ));
                                                            let sample: Vec<String> = all
                                                                .iter()
                                                                .take(5)
                                                                .cloned()
                                                                .collect();
                                                            debug_log(&format!(
                                                                "switch group_nodes_sample={:?}",
                                                                sample
                                                            ));
                                                        }
                                                    }
                                                    debug_log(&format!(
                                                        "switch proxies_count={}",
                                                        state.clash_state.proxies.len()
                                                    ));
                                                    refresh_update_providers(
                                                        state,
                                                        config,
                                                        &mut update_providers,
                                                    )
                                                    .await;
                                                    routes_expanded = false;
                                                    selected_route_index = 0;
                                                    selected_node_index = 0;
                                                    update_selected_index = update_selected_index
                                                        .min(
                                                            update_providers
                                                                .len()
                                                                .saturating_sub(1),
                                                        );
                                                    last_refresh = std::time::Instant::now();
                                                    let status =
                                                        if let Some(count) = applied_proxy_count {
                                                            format!(
                                                            "Switched to {} ({} proxies, {} rules)",
                                                            item.name,
                                                            count,
                                                            rules_data.len()
                                                        )
                                                        } else {
                                                            format!(
                                                                "Switched to {} ({} rules)",
                                                                item.name,
                                                                rules_data.len()
                                                            )
                                                        };
                                                    state.status_message = Some(status);
                                                }
                                                Err(e) => {
                                                    state.status_message = Some(format!(
                                                        "Failed to reload Clash config: {}",
                                                        e
                                                    ));
                                                    debug_log(&format!(
                                                        "switch reload failed: {}",
                                                        e
                                                    ));
                                                }
                                            }
                                        }
                                        _ => {
                                            state.status_message = Some(
                                                "Only Mihomo Party profiles support switching"
                                                    .to_string(),
                                            );
                                        }
                                    }
                                } else {
                                    state.status_message =
                                        Some("No subscriptions to switch".to_string());
                                }
                            }
                            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                state.preset = state.preset.next();
                                state.mode = state.preset.default_mode();
                                state.status_message = Some(format!(
                                    "Switched to {} preset: {}",
                                    state.preset.name(),
                                    state.preset.description()
                                ));
                            }
                            KeyCode::Char('u') => {
                                // Update all providers
                                if update_in_flight > 0 {
                                    state.status_message =
                                        Some("Update in progress...".to_string());
                                } else if update_providers.is_empty() {
                                    state.status_message =
                                        Some("No subscriptions to update".to_string());
                                } else {
                                    update_total = update_providers.len();
                                    update_in_flight = update_total;
                                    update_success = 0;
                                    update_fail = 0;
                                    state.status_message =
                                        Some(format!("Updating... (0/{})", update_total));

                                    for (idx, item) in update_providers.iter().cloned().enumerate()
                                    {
                                        spawn_update_task(
                                            update_tx.clone(),
                                            item,
                                            idx,
                                            state.clash_state.client.clone(),
                                        );
                                    }
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
                                    state.status_message =
                                        Some("Refreshing connections...".to_string());
                                    match state.clash_state.client.get_connections().await {
                                        Ok(data) => {
                                            connections_data = Some(data);
                                            state.status_message =
                                                Some("Connections refreshed!".to_string());
                                        }
                                        Err(e) => {
                                            state.status_message =
                                                Some(format!("Failed to refresh: {}", e));
                                        }
                                    }
                                    connections_last_refresh = std::time::Instant::now();
                                }
                                KeyCode::Up => {
                                    connections_selected_index =
                                        connections_selected_index.saturating_sub(1);
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
                                            if connections_selected_index
                                                >= connections_scroll_offset + visible_items
                                            {
                                                connections_scroll_offset =
                                                    connections_selected_index - visible_items + 1;
                                            }
                                        }
                                    }
                                }
                                KeyCode::Char('d') | KeyCode::Char('D') => {
                                    // Close selected connection
                                    if let Some(conn) = &connections_data {
                                        if connections_selected_index < conn.connections.len() {
                                            let connection_id = conn.connections
                                                [connections_selected_index]
                                                .id
                                                .clone();
                                            state.status_message = Some(format!(
                                                "Closing connection {}...",
                                                connection_id
                                            ));
                                            match state
                                                .clash_state
                                                .client
                                                .close_connection(&connection_id)
                                                .await
                                            {
                                                Ok(_) => {
                                                    state.status_message =
                                                        Some("Connection closed!".to_string());
                                                    // Refresh connections
                                                    if let Ok(data) = state
                                                        .clash_state
                                                        .client
                                                        .get_connections()
                                                        .await
                                                    {
                                                        connections_data = Some(data);
                                                        // Adjust selected index if needed
                                                        if let Some(conn) = &connections_data {
                                                            if connections_selected_index
                                                                >= conn.connections.len()
                                                                && conn.connections.len() > 0
                                                            {
                                                                connections_selected_index =
                                                                    conn.connections.len() - 1;
                                                            }
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    state.status_message = Some(format!(
                                                        "Failed to close connection: {}",
                                                        e
                                                    ));
                                                }
                                            }
                                            connections_last_refresh = std::time::Instant::now();
                                        }
                                    }
                                }
                                KeyCode::Char('a') | KeyCode::Char('A') => {
                                    // Close all connections
                                    state.status_message =
                                        Some("Closing all connections...".to_string());
                                    match state.clash_state.client.close_all_connections().await {
                                        Ok(_) => {
                                            state.status_message =
                                                Some("All connections closed!".to_string());
                                            // Refresh connections
                                            if let Ok(data) =
                                                state.clash_state.client.get_connections().await
                                            {
                                                connections_data = Some(data);
                                                connections_selected_index = 0;
                                            }
                                        }
                                        Err(e) => {
                                            state.status_message = Some(format!(
                                                "Failed to close all connections: {}",
                                                e
                                            ));
                                        }
                                    }
                                    connections_last_refresh = std::time::Instant::now();
                                }
                                KeyCode::Char('c')
                                    if key.modifiers.contains(KeyModifiers::CONTROL) =>
                                {
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
                                            .unwrap_or_else(|| {
                                                std::path::PathBuf::from("clashctl-export.yaml")
                                            });

                                        match config.export_to(&export_path) {
                                            Ok(_) => {
                                                settings_action =
                                                    pages::SettingsAction::ExportSuccess(
                                                        export_path.display().to_string(),
                                                    );
                                            }
                                            Err(e) => {
                                                settings_action = pages::SettingsAction::Error(
                                                    format!("Export failed: {}", e),
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
                                            .unwrap_or_else(|| {
                                                std::path::PathBuf::from("clashctl-import.yaml")
                                            });

                                        match AppConfig::import_from(&import_path) {
                                            Ok(imported_config) => {
                                                // Save imported config
                                                if let Err(e) = imported_config.save() {
                                                    settings_action =
                                                        pages::SettingsAction::Error(format!(
                                                            "Failed to save imported config: {}",
                                                            e
                                                        ));
                                                } else {
                                                    *config = imported_config;
                                                    settings_action =
                                                        pages::SettingsAction::ImportSuccess;
                                                }
                                            }
                                            Err(e) => {
                                                settings_action = pages::SettingsAction::Error(
                                                    format!("Import failed: {}", e),
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
                                    KeyCode::Char('c')
                                        if key.modifiers.contains(KeyModifiers::CONTROL) =>
                                    {
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
                                    stop_logs_stream(&mut logs_shutdown, &mut logs_task);
                                    logs_connected = false;
                                    logs_status_detail = None;
                                    state.current_page = Page::Home;
                                }
                                KeyCode::Char('h') => {
                                    stop_logs_stream(&mut logs_shutdown, &mut logs_task);
                                    logs_connected = false;
                                    logs_status_detail = None;
                                    state.current_page = Page::Home;
                                }
                                KeyCode::Char('r') => {
                                    // Refresh logs
                                    state.status_message = Some("Reconnecting logs...".to_string());
                                    logs_data.clear();
                                    logs_scroll_offset = 0;
                                    logs_connected = false;
                                    logs_status_detail = Some("reconnecting".to_string());
                                    start_logs_stream(
                                        state.clash_state.client.clone(),
                                        log_level_to_ws(logs_level_filter),
                                        logs_tx.clone(),
                                        &mut logs_shutdown,
                                        &mut logs_task,
                                    );
                                }
                                KeyCode::Char('f') | KeyCode::Char('F') => {
                                    // Change filter level
                                    logs_level_filter = logs_level_filter.next();
                                    logs_scroll_offset = 0;
                                    state.status_message =
                                        Some(format!("Filter: {}", logs_level_filter.as_str()));
                                    logs_data.clear();
                                    logs_connected = false;
                                    logs_status_detail = Some("reconnecting".to_string());
                                    start_logs_stream(
                                        state.clash_state.client.clone(),
                                        log_level_to_ws(logs_level_filter),
                                        logs_tx.clone(),
                                        &mut logs_shutdown,
                                        &mut logs_task,
                                    );
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
                                KeyCode::Char('c')
                                    if key.modifiers.contains(KeyModifiers::CONTROL) =>
                                {
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
                                    Err(e) => {
                                        state.status_message =
                                            Some(format!("Failed to fetch connections: {}", e))
                                    }
                                }
                                connections_last_refresh = std::time::Instant::now();
                            }
                            KeyCode::Char('r') => {
                                // Manual refresh
                                state.status_message =
                                    Some("Refreshing performance data...".to_string());
                                match state.clash_state.client.get_connections().await {
                                    Ok(data) => {
                                        let elapsed_secs =
                                            performance_last_refresh.elapsed().as_secs();
                                        if elapsed_secs > 0 {
                                            performance_upload_rate = (data
                                                .upload_total
                                                .saturating_sub(performance_upload_total))
                                                / elapsed_secs;
                                            performance_download_rate = (data
                                                .download_total
                                                .saturating_sub(performance_download_total))
                                                / elapsed_secs;
                                        }
                                        performance_upload_total = data.upload_total;
                                        performance_download_total = data.download_total;
                                        performance_connection_count = data.connections.len();
                                        state.status_message =
                                            Some("Performance data refreshed!".to_string());
                                    }
                                    Err(e) => {
                                        state.status_message =
                                            Some(format!("Failed to refresh: {}", e));
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
            " v0.1.2 - Simple-first TUI Clash Controller",
            Style::default().fg(theme.text()),
        ),
        Span::styled(
            format!(" [{}]", theme.name()),
            Style::default().fg(theme.text_muted()),
        ),
    ]))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border())),
    );

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
    let clear_block = Block::default().style(Style::default().bg(Color::Black));
    f.render_widget(clear_block, dialog_area);

    // Dialog content
    let dialog_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(1), // Message
            Constraint::Length(1), // Prompt
        ])
        .split(dialog_area);

    let title = Paragraph::new("Quit Confirmation")
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(ratatui::style::Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, dialog_chunks[0]);

    let message = Paragraph::new("Are you sure you want to quit?").alignment(Alignment::Center);
    f.render_widget(message, dialog_chunks[1]);

    let prompt = Paragraph::new(Line::from(vec![
        Span::styled(
            "Y",
            Style::default()
                .fg(Color::Green)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ),
        Span::raw("es / "),
        Span::styled(
            "N",
            Style::default()
                .fg(Color::Red)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ),
        Span::raw("o"),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(prompt, dialog_chunks[2]);
}
