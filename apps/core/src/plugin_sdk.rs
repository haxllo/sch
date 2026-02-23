use crate::config::Config;
use crate::model::SearchItem;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginActionKind {
    OpenPath { path: String },
    Command { command: String, args: Vec<String> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginAction {
    pub result_id: String,
    pub plugin_id: String,
    pub action_id: String,
    pub title: String,
    pub subtitle: String,
    pub keywords: Vec<String>,
    pub kind: PluginActionKind,
}

#[derive(Debug, Default, Clone)]
pub struct PluginRegistry {
    pub provider_items: Vec<SearchItem>,
    pub action_items: Vec<SearchItem>,
    pub actions_by_result_id: HashMap<String, PluginAction>,
    pub load_warnings: Vec<String>,
}

impl PluginRegistry {
    pub fn load_from_config(cfg: &Config) -> Self {
        if !cfg.plugins_enabled {
            return Self::default();
        }

        let mut registry = Self::default();
        for path in &cfg.plugin_paths {
            for manifest_path in discover_manifest_paths(path) {
                match load_manifest(&manifest_path) {
                    Ok(manifest) => append_manifest(&mut registry, manifest),
                    Err(error) => registry.load_warnings.push(format!(
                        "plugin manifest '{}' failed: {error}",
                        manifest_path.display()
                    )),
                }
            }
        }
        registry
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct PluginManifest {
    id: String,
    name: String,
    version: String,
    enabled: bool,
    provider_items: Vec<ManifestProviderItem>,
    actions: Vec<ManifestAction>,
}

impl Default for PluginManifest {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            version: String::new(),
            enabled: true,
            provider_items: Vec::new(),
            actions: Vec::new(),
        }
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct ManifestProviderItem {
    id: String,
    kind: String,
    title: String,
    path: String,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct ManifestAction {
    id: String,
    title: String,
    subtitle: String,
    keywords: Vec<String>,
    #[serde(rename = "type")]
    action_type: String,
    path: String,
    command: String,
    args: Vec<String>,
}

impl Default for ManifestAction {
    fn default() -> Self {
        Self {
            id: String::new(),
            title: String::new(),
            subtitle: String::new(),
            keywords: Vec::new(),
            action_type: "open_path".to_string(),
            path: String::new(),
            command: String::new(),
            args: Vec::new(),
        }
    }
}

fn discover_manifest_paths(path: &Path) -> Vec<PathBuf> {
    if path.is_file() {
        return vec![path.to_path_buf()];
    }
    if !path.is_dir() {
        return Vec::new();
    }

    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let entry_path = entry.path();
            if entry_path.is_file()
                && entry_path
                    .extension()
                    .and_then(|v| v.to_str())
                    .is_some_and(|v| v.eq_ignore_ascii_case("json"))
            {
                out.push(entry_path);
            }
        }
    }
    out
}

fn load_manifest(path: &Path) -> Result<PluginManifest, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| format!("read failed for '{}': {e}", path.display()))?;
    let manifest: PluginManifest = serde_json::from_str(&raw)
        .map_err(|e| format!("invalid json in '{}': {e}", path.display()))?;
    if manifest.id.trim().is_empty() {
        return Err("missing plugin id".to_string());
    }
    Ok(manifest)
}

fn append_manifest(registry: &mut PluginRegistry, manifest: PluginManifest) {
    if !manifest.enabled {
        return;
    }

    let plugin_id = manifest.id.trim().to_string();
    let plugin_label = if manifest.name.trim().is_empty() {
        plugin_id.clone()
    } else {
        manifest.name.trim().to_string()
    };

    for item in manifest.provider_items {
        let item_id = item.id.trim();
        let title = item.title.trim();
        if item_id.is_empty() || title.is_empty() {
            continue;
        }
        let result_id = format!("plugin:{plugin_id}:item:{item_id}");
        let kind = if item.kind.trim().is_empty() {
            "file".to_string()
        } else {
            item.kind.trim().to_string()
        };
        registry
            .provider_items
            .push(SearchItem::new(&result_id, &kind, title, item.path.trim()));
    }

    for action in manifest.actions {
        let action_id = action.id.trim();
        let action_title = action.title.trim();
        if action_id.is_empty() || action_title.is_empty() {
            continue;
        }
        let result_id = format!("plugin:{plugin_id}:action:{action_id}");
        let subtitle = if action.subtitle.trim().is_empty() {
            format!("{plugin_label} plugin action")
        } else {
            action.subtitle.trim().to_string()
        };
        let kind = parse_action_kind(&action);
        let plugin_action = PluginAction {
            result_id: result_id.clone(),
            plugin_id: plugin_id.clone(),
            action_id: action_id.to_string(),
            title: action_title.to_string(),
            subtitle: subtitle.clone(),
            keywords: action.keywords,
            kind,
        };
        let keyword_suffix = if plugin_action.keywords.is_empty() {
            String::new()
        } else {
            format!(" {}", plugin_action.keywords.join(" "))
        };
        registry.action_items.push(SearchItem::new(
            &result_id,
            "action",
            action_title,
            &format!("{subtitle}{keyword_suffix}"),
        ));
        registry
            .actions_by_result_id
            .insert(result_id, plugin_action);
    }
}

fn parse_action_kind(action: &ManifestAction) -> PluginActionKind {
    let normalized = action.action_type.trim().to_ascii_lowercase();
    if normalized == "command" {
        return PluginActionKind::Command {
            command: action.command.trim().to_string(),
            args: action.args.clone(),
        };
    }
    PluginActionKind::OpenPath {
        path: action.path.trim().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_action_kind;
    use super::{ManifestAction, PluginActionKind};

    #[test]
    fn parses_command_action_kind() {
        let action = ManifestAction {
            action_type: "command".to_string(),
            command: "cmd".to_string(),
            args: vec!["/C".to_string(), "echo".to_string()],
            ..Default::default()
        };
        let kind = parse_action_kind(&action);
        assert!(matches!(kind, PluginActionKind::Command { .. }));
    }
}
