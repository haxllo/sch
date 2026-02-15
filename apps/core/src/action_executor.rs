use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchError {
    EmptyPath,
    MissingPath(PathBuf),
    LaunchFailed {
        message: String,
        code: Option<i32>,
    },
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

#[cfg(target_os = "windows")]
fn launch_existing_path(candidate: &Path) -> Result<(), LaunchError> {
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let target = candidate.to_string_lossy().into_owned();
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

#[cfg(target_os = "windows")]
fn to_wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}
