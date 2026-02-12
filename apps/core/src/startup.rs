use std::fmt::{Display, Formatter};
use std::path::Path;

#[derive(Debug)]
pub enum StartupError {
    Io(std::io::Error),
    Command(String),
    UnsupportedPlatform,
}

impl Display for StartupError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "io error: {error}"),
            Self::Command(error) => write!(f, "command error: {error}"),
            Self::UnsupportedPlatform => write!(f, "unsupported platform"),
        }
    }
}

impl std::error::Error for StartupError {}

impl From<std::io::Error> for StartupError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

#[cfg(target_os = "windows")]
const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
#[cfg(target_os = "windows")]
const VALUE_NAME: &str = "SwiftFind";

#[cfg(target_os = "windows")]
pub fn is_enabled() -> Result<bool, StartupError> {
    let output = std::process::Command::new("reg")
        .args(["query", RUN_KEY, "/v", VALUE_NAME])
        .output()?;

    Ok(output.status.success())
}

#[cfg(target_os = "windows")]
pub fn set_enabled(enabled: bool, executable_path: &Path) -> Result<(), StartupError> {
    if enabled {
        let value = format!("\"{}\"", executable_path.to_string_lossy());
        let output = std::process::Command::new("reg")
            .args([
                "add",
                RUN_KEY,
                "/v",
                VALUE_NAME,
                "/t",
                "REG_SZ",
                "/d",
                &value,
                "/f",
            ])
            .output()?;
        if output.status.success() {
            return Ok(());
        }
        return Err(StartupError::Command(String::from_utf8_lossy(&output.stderr).trim().to_string()));
    }

    let output = std::process::Command::new("reg")
        .args(["delete", RUN_KEY, "/v", VALUE_NAME, "/f"])
        .output()?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).to_ascii_lowercase();
    if stderr.contains("unable to find") || stderr.contains("cannot find") {
        return Ok(());
    }

    Err(StartupError::Command(stderr.trim().to_string()))
}

#[cfg(not(target_os = "windows"))]
pub fn is_enabled() -> Result<bool, StartupError> {
    Err(StartupError::UnsupportedPlatform)
}

#[cfg(not(target_os = "windows"))]
pub fn set_enabled(_enabled: bool, _executable_path: &Path) -> Result<(), StartupError> {
    Err(StartupError::UnsupportedPlatform)
}
