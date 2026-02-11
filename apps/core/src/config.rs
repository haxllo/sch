use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const CURRENT_CONFIG_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Config {
    pub version: u32,
    pub max_results: u16,
    pub index_db_path: PathBuf,
    pub config_path: PathBuf,
    pub discovery_roots: Vec<PathBuf>,
    pub hotkey: String,
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
            hotkey: "Alt+Space".to_string(),
        }
    }
}

#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Parse(serde_json::Error),
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
        Self::Parse(value)
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
    let resolved_path = path.map(Path::to_path_buf).unwrap_or_else(stable_config_path);

    if !resolved_path.exists() {
        let mut cfg = Config::default();
        cfg.config_path = resolved_path.clone();
        if cfg.index_db_path == Config::default().index_db_path {
            cfg.index_db_path = resolved_path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join("index.sqlite3");
        }
        validate(&cfg).map_err(ConfigError::Validation)?;
        return Ok(cfg);
    }

    let raw = std::fs::read_to_string(&resolved_path)?;
    let mut cfg: Config = serde_json::from_str(&raw)?;
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
    std::fs::write(path, encoded)?;
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

    if cfg.version == 0 {
        return Err("version must be >= 1".into());
    }

    Ok(())
}

fn default_discovery_roots() -> Vec<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        if let Ok(user_profile) = std::env::var("USERPROFILE") {
            let base = PathBuf::from(user_profile);
            return vec![base.join("Documents"), base.join("Desktop")];
        }
    }

    Vec::new()
}
