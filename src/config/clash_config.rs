use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

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
        Self::find_config_with_hint(None)
    }

    /// Try to find Clash config with an optional hint path
    pub fn find_config_with_hint(hint: Option<&Path>) -> Option<PathBuf> {
        if let Some(path) = config_path_from_env("CLASH_CONFIG_PATH") {
            return Some(path);
        }

        if let Some(path) = config_path_from_env("CLASH_PARTY_DIR") {
            return Some(path);
        }

        if let Some(hint) = hint {
            if hint.is_file() {
                return Some(hint.to_path_buf());
            }

            if hint.is_dir() {
                if let Some(found) = config_from_dir(hint) {
                    return Some(found);
                }
            }
        }

        let mut possible_paths = Vec::new();

        if let Some(home) = dirs::home_dir() {
            add_candidate_dir(
                &mut possible_paths,
                &home.join("Library/Application Support/mihomo-party"),
            );
            add_candidate_dir(
                &mut possible_paths,
                &home.join("Library/Application Support/Clash Verge/mihomo-party"),
            );
            add_candidate_dir(&mut possible_paths, &home.join(".config/clash"));
            add_candidate_dir(&mut possible_paths, &home.join(".config/mihomo"));
            add_candidate_dir(&mut possible_paths, &home.join(".config/mihomo-party"));
            add_candidate_dir(
                &mut possible_paths,
                &home.join(".config/clash-verge/mihomo-party"),
            );
            add_candidate_dir(
                &mut possible_paths,
                &home.join("AppData/Roaming/mihomo-party"),
            );
            add_candidate_dir(
                &mut possible_paths,
                &home.join("AppData/Roaming/Clash Verge/mihomo-party"),
            );
        }

        possible_paths.push(PathBuf::from("/etc/clash/config.yaml"));
        possible_paths.push(PathBuf::from("/etc/clash/config.yml"));

        for path in possible_paths {
            if path.is_file() && is_probable_clash_config(&path) {
                return Some(path);
            }
        }

        let mut scan_roots = Vec::new();
        if let Some(home) = dirs::home_dir() {
            scan_roots.push(home.join("Library/Application Support"));
            scan_roots.push(home.join(".config"));
            scan_roots.push(home.join("AppData/Roaming"));
        }

        for root in scan_roots {
            if root.is_dir() {
                if let Some(found) = scan_for_config(&root, 3) {
                    return Some(found);
                }
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

fn config_path_from_env(var: &str) -> Option<PathBuf> {
    let raw = std::env::var_os(var)?;
    let path = PathBuf::from(raw);
    if path.is_file() {
        return Some(path);
    }

    if path.is_dir() {
        if let Some(found) = config_from_dir(&path) {
            return Some(found);
        }
        return scan_for_config(&path, 3);
    }

    None
}

fn add_candidate_dir(possible_paths: &mut Vec<PathBuf>, dir: &Path) {
    possible_paths.push(dir.join("config.yaml"));
    possible_paths.push(dir.join("config.yml"));
}

fn config_from_dir(dir: &Path) -> Option<PathBuf> {
    let yaml = dir.join("config.yaml");
    if yaml.is_file() {
        return Some(yaml);
    }

    let yml = dir.join("config.yml");
    if yml.is_file() {
        return Some(yml);
    }

    None
}

fn scan_for_config(root: &Path, max_depth: usize) -> Option<PathBuf> {
    let mut stack = vec![(root.to_path_buf(), 0usize)];
    let mut candidates = Vec::new();

    while let Some((dir, depth)) = stack.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if depth < max_depth && !should_skip_dir(&path) {
                    stack.push((path, depth + 1));
                }
                continue;
            }

            if !is_config_filename(&path) {
                continue;
            }

            if !looks_like_clash_path(&path) {
                continue;
            }

            if is_probable_clash_config(&path) {
                candidates.push(path);
            }
        }
    }

    select_most_recent(candidates)
}

fn should_skip_dir(path: &Path) -> bool {
    let name = match path.file_name().and_then(|n| n.to_str()) {
        Some(name) => name.to_lowercase(),
        None => return false,
    };

    matches!(
        name.as_str(),
        ".git" | "node_modules" | "cache" | "caches" | "tmp" | "temp"
    )
}

fn is_config_filename(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|n| n.to_str()),
        Some("config.yaml") | Some("config.yml")
    )
}

fn looks_like_clash_path(path: &Path) -> bool {
    let lower = path.to_string_lossy().to_lowercase();
    lower.contains("clash")
        || lower.contains("mihomo")
        || lower.contains("verge")
        || lower.contains("party")
}

fn is_probable_clash_config(path: &Path) -> bool {
    let content = match fs::read_to_string(path) {
        Ok(content) => content.to_lowercase(),
        Err(_) => return false,
    };

    content.contains("proxy-providers")
        || content.contains("proxies:")
        || content.contains("external-controller")
        || content.contains("mixed-port")
        || content.contains("socks-port")
}

fn select_most_recent(candidates: Vec<PathBuf>) -> Option<PathBuf> {
    let mut best: Option<(SystemTime, PathBuf)> = None;

    for path in candidates {
        let modified = fs::metadata(&path)
            .and_then(|meta| meta.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);

        let is_better = match &best {
            Some((best_time, _)) => modified > *best_time,
            None => true,
        };

        if is_better {
            best = Some((modified, path));
        }
    }

    best.map(|(_, path)| path)
}
