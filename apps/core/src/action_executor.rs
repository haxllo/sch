pub fn launch_path(path: &str) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err("empty path".into());
    }
    Ok(())
}
