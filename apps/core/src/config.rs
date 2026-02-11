#[derive(Default)]
pub struct Config {
    pub max_results: u16,
}

pub fn validate(cfg: &Config) -> Result<(), String> {
    if cfg.max_results < 5 || cfg.max_results > 100 {
        return Err("max_results out of range".into());
    }
    Ok(())
}
