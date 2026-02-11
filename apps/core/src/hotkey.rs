#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hotkey {
    pub modifiers: Vec<String>,
    pub key: String,
}

pub fn parse_hotkey(input: &str) -> Result<Hotkey, String> {
    let parts: Vec<&str> = input.split('+').collect();
    if parts.len() < 2 {
        return Err("invalid hotkey".into());
    }

    Ok(Hotkey {
        modifiers: parts[..parts.len() - 1]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        key: parts[parts.len() - 1].to_string(),
    })
}
