use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchError {
    EmptyPath,
    MissingPath(PathBuf),
}

impl Display for LaunchError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyPath => write!(f, "empty path"),
            Self::MissingPath(path) => write!(f, "path does not exist: {}", path.display()),
        }
    }
}

impl std::error::Error for LaunchError {}

pub fn launch_path(path: &str) -> Result<(), LaunchError> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(LaunchError::EmptyPath);
    }

    let candidate = Path::new(trimmed);
    if !candidate.exists() {
        return Err(LaunchError::MissingPath(candidate.to_path_buf()));
    }

    Ok(())
}
