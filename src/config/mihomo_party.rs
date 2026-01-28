use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MihomoPartyProfileList {
    #[serde(default)]
    pub items: Vec<MihomoPartyProfileItem>,
    #[serde(default)]
    pub current: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_yaml::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MihomoPartyProfileItem {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub profile_type: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub updated: Option<i64>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_yaml::Value>,
}

impl MihomoPartyProfileList {
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let list: MihomoPartyProfileList = serde_yaml::from_str(&content)?;
        Ok(list)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let content = serde_yaml::to_string(self)?;
        fs::write(path, content)?;
        Ok(())
    }
}

pub fn find_profile_list_with_hint(hint: Option<&Path>) -> Option<PathBuf> {
    if let Some(path) = profile_path_from_env("CLASH_PARTY_DIR") {
        return Some(path);
    }

    if let Some(hint) = hint {
        if let Some(found) = resolve_profile_list_from_hint(hint) {
            return Some(found);
        }
    }

    let mut possible_paths = Vec::new();
    if let Some(home) = dirs::home_dir() {
        possible_paths.push(home.join("Library/Application Support/mihomo-party/profile.yaml"));
        possible_paths
            .push(home.join("Library/Application Support/Clash Verge/mihomo-party/profile.yaml"));
        possible_paths.push(home.join(".config/mihomo-party/profile.yaml"));
        possible_paths.push(home.join(".config/clash-verge/mihomo-party/profile.yaml"));
        possible_paths.push(home.join("AppData/Roaming/mihomo-party/profile.yaml"));
        possible_paths.push(home.join("AppData/Roaming/Clash Verge/mihomo-party/profile.yaml"));
    }

    for path in possible_paths {
        if path.is_file() {
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
            if let Some(found) = scan_for_profile_list(&root, 3) {
                return Some(found);
            }
        }
    }

    None
}

pub fn profile_path_from_list(list_path: &Path, id: &str) -> Option<PathBuf> {
    let root = list_path.parent()?;
    Some(root.join("profiles").join(format!("{id}.yaml")))
}

pub fn update_profile_updated_at(list_path: &Path, id: &str, updated_at_ms: i64) -> Result<()> {
    let mut list = MihomoPartyProfileList::load(list_path)?;
    if let Some(item) = list.items.iter_mut().find(|item| item.id == id) {
        item.updated = Some(updated_at_ms);
    }
    list.save(list_path)
}

pub fn count_proxies_in_profile(path: &Path) -> Option<usize> {
    let content = fs::read_to_string(path).ok()?;
    let value: serde_yaml::Value = serde_yaml::from_str(&content).ok()?;
    value
        .get("proxies")
        .and_then(|v| v.as_sequence())
        .map(|seq| seq.len())
}

fn profile_path_from_env(var: &str) -> Option<PathBuf> {
    let raw = std::env::var_os(var)?;
    let path = PathBuf::from(raw);
    if path.is_file() {
        return Some(path);
    }

    if path.is_dir() {
        let candidate = path.join("profile.yaml");
        if candidate.is_file() {
            return Some(candidate);
        }
        return scan_for_profile_list(&path, 3);
    }

    None
}

fn resolve_profile_list_from_hint(hint: &Path) -> Option<PathBuf> {
    if hint.is_file() {
        if let Some(parent) = hint.parent() {
            let candidate = parent.join("profile.yaml");
            if candidate.is_file() {
                return Some(candidate);
            }
        }

        for ancestor in hint.ancestors() {
            if ancestor.file_name().and_then(|n| n.to_str()) == Some("mihomo-party") {
                let candidate = ancestor.join("profile.yaml");
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }

    if hint.is_dir() {
        let candidate = hint.join("profile.yaml");
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    None
}

fn scan_for_profile_list(root: &Path, max_depth: usize) -> Option<PathBuf> {
    let mut stack = vec![(root.to_path_buf(), 0usize)];

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

            if path.file_name().and_then(|n| n.to_str()) != Some("profile.yaml") {
                continue;
            }

            return Some(path);
        }
    }

    None
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
