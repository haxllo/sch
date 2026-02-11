use std::path::PathBuf;

pub struct Config {
    pub max_results: u16,
    pub index_db_path: PathBuf,
    pub config_path: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        let base = std::env::temp_dir().join("swiftfind");
        Self {
            max_results: 20,
            index_db_path: base.join("index.sqlite3"),
            config_path: base.join("config.toml"),
        }
    }
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

    Ok(())
}
