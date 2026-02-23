use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchError {
    EmptyPath,
    MissingPath(PathBuf),
    LaunchFailed { message: String, code: Option<i32> },
}

impl Display for LaunchError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyPath => write!(f, "empty path"),
            Self::MissingPath(path) => write!(f, "path does not exist: {}", path.display()),
            Self::LaunchFailed { message, code } => {
                if let Some(code) = code {
                    write!(f, "launch failed: {message} (code {code})")
                } else {
                    write!(f, "launch failed: {message}")
                }
            }
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

    launch_existing_path(candidate)?;

    Ok(())
}

pub fn launch_open_target(target: &str) -> Result<(), LaunchError> {
    let trimmed = target.trim();
    if trimmed.is_empty() {
        return Err(LaunchError::EmptyPath);
    }
    launch_open(trimmed)
}

pub fn launch_browser_default_search(
    query: &str,
    fallback_url: Option<&str>,
) -> Result<(), LaunchError> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Err(LaunchError::EmptyPath);
    }

    if let Some(url) = fallback_url {
        let fallback = url.trim();
        if !fallback.is_empty() {
            return launch_open(fallback);
        }
    }

    Err(LaunchError::LaunchFailed {
        message: "missing fallback web search URL".to_string(),
        code: None,
    })
}

#[cfg(target_os = "windows")]
fn launch_existing_path(candidate: &Path) -> Result<(), LaunchError> {
    let target = candidate.to_string_lossy().into_owned();
    launch_open(&target)
}

#[cfg(target_os = "windows")]
fn launch_open(target: &str) -> Result<(), LaunchError> {
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let wide_target = to_wide(&target);
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            std::ptr::null(),
            wide_target.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    } as isize;

    if result <= 32 {
        return Err(LaunchError::LaunchFailed {
            message: format!("ShellExecuteW failed for '{target}'"),
            code: Some(result as i32),
        });
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn launch_existing_path(_candidate: &Path) -> Result<(), LaunchError> {
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn launch_open(_target: &str) -> Result<(), LaunchError> {
    Ok(())
}

#[cfg(target_os = "windows")]
fn to_wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}
