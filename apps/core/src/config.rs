pub struct Config {
    pub max_results: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self { max_results: 20 }
    }
}

pub fn validate(cfg: &Config) -> Result<(), String> {
    if cfg.max_results < 5 || cfg.max_results > 100 {
        return Err("max_results out of range".into());
    }
    Ok(())
}
