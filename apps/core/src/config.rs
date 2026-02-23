use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

pub const CURRENT_CONFIG_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SearchMode {
    #[default]
    All,
    Apps,
    Files,
    Actions,
    Clipboard,
}

impl SearchMode {
    pub fn parse(value: &str) -> Option<Self> {
        let normalized = value.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "all" => Some(Self::All),
            "apps" | "app" => Some(Self::Apps),
            "files" | "file" => Some(Self::Files),
            "actions" | "action" => Some(Self::Actions),
            "clipboard" | "clip" => Some(Self::Clipboard),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Config {
    pub version: u32,
    pub max_results: u16,
    pub index_db_path: PathBuf,
    pub config_path: PathBuf,
    pub discovery_roots: Vec<PathBuf>,
    pub discovery_exclude_roots: Vec<PathBuf>,
    pub hotkey: String,
    pub launch_at_startup: bool,
    pub hotkey_help: String,
    pub hotkey_recommended: Vec<String>,
    pub search_mode_default: SearchMode,
    pub search_dsl_enabled: bool,
    pub clipboard_enabled: bool,
    pub clipboard_retention_minutes: u32,
    pub clipboard_exclude_sensitive_patterns: Vec<String>,
    pub plugins_enabled: bool,
    pub plugin_paths: Vec<PathBuf>,
    pub plugins_safe_mode: bool,
    pub idle_cache_trim_ms: u32,
    pub active_memory_target_mb: u16,
}

impl Default for Config {
    fn default() -> Self {
        let app_dir = stable_app_data_dir();
        let config_path = app_dir.join("config.json");
        Self {
            version: CURRENT_CONFIG_VERSION,
            max_results: 20,
            index_db_path: app_dir.join("index.sqlite3"),
            config_path,
            discovery_roots: default_discovery_roots(),
            discovery_exclude_roots: default_discovery_exclude_roots(),
            hotkey: "Ctrl+Shift+Space".to_string(),
            launch_at_startup: false,
            hotkey_help:
                "Set `hotkey` as Modifier+Modifier+Key (example: Ctrl+Shift+Space), then restart SwiftFind."
                    .to_string(),
            hotkey_recommended: vec![
                "Ctrl+Shift+Space".to_string(),
                "Ctrl+Alt+Space".to_string(),
                "Alt+Shift+Space".to_string(),
                "Ctrl+Shift+P".to_string(),
                "Ctrl+Alt+P".to_string(),
            ],
            search_mode_default: SearchMode::All,
            search_dsl_enabled: true,
            clipboard_enabled: true,
            clipboard_retention_minutes: 8 * 60,
            clipboard_exclude_sensitive_patterns: vec![
                "password".to_string(),
                "passcode".to_string(),
                "otp".to_string(),
                "token".to_string(),
                "secret".to_string(),
                "apikey".to_string(),
                "api_key".to_string(),
            ],
            plugins_enabled: true,
            plugin_paths: vec![app_dir.join("plugins")],
            plugins_safe_mode: true,
            idle_cache_trim_ms: 1200,
            active_memory_target_mb: 80,
        }
    }
}

#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Parse(String),
    Validation(String),
}

impl Display for ConfigError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "io error: {error}"),
            Self::Parse(error) => write!(f, "parse error: {error}"),
            Self::Validation(error) => write!(f, "validation error: {error}"),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<std::io::Error> for ConfigError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for ConfigError {
    fn from(value: serde_json::Error) -> Self {
        Self::Parse(value.to_string())
    }
}

pub fn stable_app_data_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Ok(app_data) = std::env::var("APPDATA") {
            return PathBuf::from(app_data).join("SwiftFind");
        }

        if let Ok(user_profile) = std::env::var("USERPROFILE") {
            return PathBuf::from(user_profile)
                .join("AppData")
                .join("Roaming")
                .join("SwiftFind");
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            return PathBuf::from(xdg).join("swiftfind");
        }

        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(".config").join("swiftfind");
        }
    }

    std::env::temp_dir().join("swiftfind")
}

pub fn stable_config_path() -> PathBuf {
    stable_app_data_dir().join("config.json")
}

pub fn load(path: Option<&Path>) -> Result<Config, ConfigError> {
    let resolved_path = path
        .map(Path::to_path_buf)
        .unwrap_or_else(stable_config_path);

    if !resolved_path.exists() {
        let cfg = default_for_path(&resolved_path);
        validate(&cfg).map_err(ConfigError::Validation)?;
        return Ok(cfg);
    }

    let raw = std::fs::read_to_string(&resolved_path)?;
    let mut cfg: Config = parse_text(&raw)?;
    cfg.config_path = resolved_path.clone();

    if cfg.index_db_path.as_os_str().is_empty() {
        cfg.index_db_path = resolved_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("index.sqlite3");
    }

    validate(&cfg).map_err(ConfigError::Validation)?;
    Ok(cfg)
}

pub fn save(cfg: &Config) -> Result<(), ConfigError> {
    save_to_path(cfg, &cfg.config_path)
}

pub fn save_to_path(cfg: &Config, path: &Path) -> Result<(), ConfigError> {
    validate(cfg).map_err(ConfigError::Validation)?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let encoded = serde_json::to_string_pretty(cfg)?;
    write_atomic(path, &encoded)
}

pub fn write_user_template(cfg: &Config, path: &Path) -> Result<(), ConfigError> {
    validate(cfg).map_err(ConfigError::Validation)?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let roots_section = json5_path_array_section(&cfg.discovery_roots);
    let excluded_roots_section = json5_path_array_section(&cfg.discovery_exclude_roots);

    let mut text = String::new();
    text.push_str("{\n");
    text.push_str("  // SwiftFind config (comments are allowed).\n");
    text.push_str("  //\n");
    text.push_str("  // Quick setup:\n");
    text.push_str("  // 1) Keep exactly ONE `hotkey` line uncommented.\n");
    text.push_str("  // 2) Save file.\n");
    text.push_str("  // 3) Restart SwiftFind.\n");
    text.push_str("  //\n");
    text.push_str("  // Safer Windows-friendly hotkeys (uncomment one if you prefer):\n");

    for option in &cfg.hotkey_recommended {
        if option != &cfg.hotkey {
            text.push_str("  // \"hotkey\": ");
            text.push_str(&json_string(option));
            text.push_str(",\n");
        }
    }

    text.push_str(
        "  // Avoid common OS-reserved/conflicting shortcuts like Win+..., Alt+Tab, Ctrl+Esc.\n",
    );
    text.push_str("  \"hotkey\": ");
    text.push_str(&json_string(&cfg.hotkey));
    text.push_str(",\n");
    text.push_str("  // Start SwiftFind automatically when you sign in (true/false)\n");
    text.push_str("  \"launch_at_startup\": ");
    text.push_str(if cfg.launch_at_startup {
        "true"
    } else {
        "false"
    });
    text.push_str(",\n\n");

    text.push_str("  // Optional tuning:\n");
    text.push_str("  // Number of results shown per query (valid range: 5..100)\n");
    text.push_str("  \"max_results\": ");
    text.push_str(&cfg.max_results.to_string());
    text.push_str(",\n\n");

    text.push_str("  // Optional: folders scanned for local files.\n");
    text.push_str("  // Add/remove paths as needed.\n");
    text.push_str("  \"discovery_roots\": ");
    text.push_str(&roots_section);
    text.push_str(",\n\n");
    text.push_str("  // Optional: folders to exclude from local-file discovery.\n");
    text.push_str("  // Any file/folder under these roots is ignored.\n");
    text.push_str("  \"discovery_exclude_roots\": ");
    text.push_str(&excluded_roots_section);
    text.push_str(",\n\n");

    text.push_str("  // Search mode default: all | apps | files | actions | clipboard\n");
    text.push_str("  \"search_mode_default\": ");
    text.push_str(&json_string(match cfg.search_mode_default {
        SearchMode::All => "all",
        SearchMode::Apps => "apps",
        SearchMode::Files => "files",
        SearchMode::Actions => "actions",
        SearchMode::Clipboard => "clipboard",
    }));
    text.push_str(",\n");
    text.push_str(
        "  // Enable query operators like kind:, modified:, created:, AND/OR/NOT and -term\n",
    );
    text.push_str("  \"search_dsl_enabled\": ");
    text.push_str(if cfg.search_dsl_enabled {
        "true"
    } else {
        "false"
    });
    text.push_str(",\n\n");

    text.push_str("  // Clipboard history provider settings\n");
    text.push_str("  \"clipboard_enabled\": ");
    text.push_str(if cfg.clipboard_enabled {
        "true"
    } else {
        "false"
    });
    text.push_str(",\n");
    text.push_str("  // Retention in minutes (valid range: 5..43200)\n");
    text.push_str("  \"clipboard_retention_minutes\": ");
    text.push_str(&cfg.clipboard_retention_minutes.to_string());
    text.push_str(",\n");
    text.push_str(
        "  // Substring patterns that should be skipped when capturing clipboard entries\n",
    );
    text.push_str("  \"clipboard_exclude_sensitive_patterns\": [\n");
    for (idx, pattern) in cfg.clipboard_exclude_sensitive_patterns.iter().enumerate() {
        text.push_str("    ");
        text.push_str(&json_string(pattern));
        if idx + 1 != cfg.clipboard_exclude_sensitive_patterns.len() {
            text.push(',');
        }
        text.push('\n');
    }
    text.push_str("  ],\n\n");

    text.push_str("  // Plugin SDK settings\n");
    text.push_str("  \"plugins_enabled\": ");
    text.push_str(if cfg.plugins_enabled { "true" } else { "false" });
    text.push_str(",\n");
    text.push_str("  // Keep safe mode true to prevent plugin command execution.\n");
    text.push_str("  \"plugins_safe_mode\": ");
    text.push_str(if cfg.plugins_safe_mode {
        "true"
    } else {
        "false"
    });
    text.push_str(",\n");
    text.push_str("  \"plugin_paths\": [\n");
    for (idx, path) in cfg.plugin_paths.iter().enumerate() {
        text.push_str("    ");
        text.push_str(&json_string(&path.to_string_lossy()));
        if idx + 1 != cfg.plugin_paths.len() {
            text.push(',');
        }
        text.push('\n');
    }
    text.push_str("  ],\n\n");

    text.push_str("  // Runtime performance targets\n");
    text.push_str("  // cache trim after hide in milliseconds (valid range: 100..10000)\n");
    text.push_str("  \"idle_cache_trim_ms\": ");
    text.push_str(&cfg.idle_cache_trim_ms.to_string());
    text.push_str(",\n");
    text.push_str("  // active memory target in MB (valid range: 20..512)\n");
    text.push_str("  \"active_memory_target_mb\": ");
    text.push_str(&cfg.active_memory_target_mb.to_string());
    text.push('\n');
    text.push_str("}\n");

    std::fs::write(path, text)?;
    Ok(())
}

pub fn validate(cfg: &Config) -> Result<(), String> {
    if cfg.max_results < 5 || cfg.max_results > 100 {
        return Err("max_results out of range".into());
    }

    if cfg.index_db_path.as_os_str().is_empty() {
        return Err("index_db_path is required".into());
    }

    if cfg.config_path.as_os_str().is_empty() {
        return Err("config_path is required".into());
    }

    if cfg.hotkey.trim().is_empty() {
        return Err("hotkey is required".into());
    }

    if cfg.clipboard_retention_minutes < 5 || cfg.clipboard_retention_minutes > 43_200 {
        return Err("clipboard_retention_minutes out of range".into());
    }

    if cfg.idle_cache_trim_ms < 100 || cfg.idle_cache_trim_ms > 10_000 {
        return Err("idle_cache_trim_ms out of range".into());
    }

    if cfg.active_memory_target_mb < 20 || cfg.active_memory_target_mb > 512 {
        return Err("active_memory_target_mb out of range".into());
    }

    if cfg
        .discovery_roots
        .iter()
        .any(|root| root.as_os_str().is_empty())
    {
        return Err("discovery_roots contains an empty path".into());
    }

    if cfg
        .discovery_exclude_roots
        .iter()
        .any(|root| root.as_os_str().is_empty())
    {
        return Err("discovery_exclude_roots contains an empty path".into());
    }

    if cfg
        .plugin_paths
        .iter()
        .any(|path| path.as_os_str().is_empty())
    {
        return Err("plugin_paths contains an empty path".into());
    }

    if cfg
        .clipboard_exclude_sensitive_patterns
        .iter()
        .any(|pattern| pattern.trim().is_empty())
    {
        return Err("clipboard_exclude_sensitive_patterns contains an empty pattern".into());
    }

    crate::settings::validate_hotkey(&cfg.hotkey)
        .map_err(|error| format!("hotkey is invalid: {error}"))?;

    if cfg.version == 0 {
        return Err("version must be >= 1".into());
    }

    Ok(())
}

fn write_atomic(path: &Path, encoded: &str) -> Result<(), ConfigError> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let temp_path = parent.join(format!(".swiftfind-config-{ts}.tmp"));
    let backup_path = parent.join(".swiftfind-config.backup");

    std::fs::write(&temp_path, encoded)?;

    if backup_path.exists() {
        let _ = std::fs::remove_file(&backup_path);
    }
    if path.exists() {
        std::fs::rename(path, &backup_path)?;
    }

    match std::fs::rename(&temp_path, path) {
        Ok(()) => {
            if backup_path.exists() {
                let _ = std::fs::remove_file(&backup_path);
            }
            Ok(())
        }
        Err(error) => {
            if backup_path.exists() {
                let _ = std::fs::rename(&backup_path, path);
            }
            let _ = std::fs::remove_file(&temp_path);
            Err(ConfigError::Io(error))
        }
    }
}

fn json5_path_array_section(paths: &[PathBuf]) -> String {
    let body = paths
        .iter()
        .map(|path| format!("    {}", json_string(&path.to_string_lossy())))
        .collect::<Vec<_>>()
        .join(",\n");

    if body.is_empty() {
        "[]".to_string()
    } else {
        format!("[\n{body}\n  ]")
    }
}

fn default_discovery_roots() -> Vec<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        if let Some(profile_root) = windows_user_profile_root() {
            return vec![profile_root];
        }
    }

    Vec::new()
}

fn default_discovery_exclude_roots() -> Vec<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        if let Some(profile_root) = windows_user_profile_root() {
            return vec![
                profile_root.join("AppData").join("Local").join("Temp"),
                profile_root
                    .join("AppData")
                    .join("Local")
                    .join("Microsoft")
                    .join("Windows")
                    .join("INetCache"),
            ];
        }
    }

    Vec::new()
}

#[cfg(target_os = "windows")]
fn windows_user_profile_root() -> Option<PathBuf> {
    if let Ok(user_profile) = std::env::var("USERPROFILE") {
        let trimmed = user_profile.trim();
        if !trimmed.is_empty() {
            return Some(PathBuf::from(trimmed));
        }
    }

    let home_drive = std::env::var("HOMEDRIVE").ok();
    let home_path = std::env::var("HOMEPATH").ok();
    if let (Some(drive), Some(path)) = (home_drive, home_path) {
        let combined = format!("{}{}", drive.trim(), path.trim());
        let trimmed = combined.trim();
        if !trimmed.is_empty() {
            return Some(PathBuf::from(trimmed));
        }
    }

    None
}

fn default_for_path(path: &Path) -> Config {
    let mut cfg = Config::default();
    cfg.config_path = path.to_path_buf();
    if cfg.index_db_path == Config::default().index_db_path {
        cfg.index_db_path = path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("index.sqlite3");
    }
    cfg
}

fn parse_text(raw: &str) -> Result<Config, ConfigError> {
    match serde_json::from_str::<Config>(raw) {
        Ok(cfg) => Ok(cfg),
        Err(json_err) => match json5::from_str::<Config>(raw) {
            Ok(cfg) => Ok(cfg),
            Err(json5_err) => Err(ConfigError::Parse(format!(
                "invalid config format. json error: {json_err}; json5 error: {json5_err}"
            ))),
        },
    }
}

fn json_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_string())
}
