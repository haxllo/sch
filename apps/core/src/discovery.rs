use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};

use crate::model::SearchItem;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderError {
    message: String,
}

impl ProviderError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for ProviderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ProviderError {}

pub trait DiscoveryProvider: Send + Sync {
    fn provider_name(&self) -> &'static str;
    fn discover(&self) -> Result<Vec<SearchItem>, ProviderError>;
}

pub struct AppProvider {
    apps: Vec<SearchItem>,
}

impl AppProvider {
    pub fn from_apps(apps: Vec<SearchItem>) -> Self {
        Self { apps }
    }

    pub fn deterministic_fixture() -> Self {
        Self {
            apps: vec![
                SearchItem::new(
                    "app-code",
                    "app",
                    "Visual Studio Code",
                    "C:\\Program Files\\Microsoft VS Code\\Code.exe",
                ),
                SearchItem::new(
                    "app-term",
                    "app",
                    "Windows Terminal",
                    "C:\\Program Files\\WindowsApps\\Terminal.exe",
                ),
            ],
        }
    }
}

impl DiscoveryProvider for AppProvider {
    fn provider_name(&self) -> &'static str {
        "app"
    }

    fn discover(&self) -> Result<Vec<SearchItem>, ProviderError> {
        Ok(self.apps.clone())
    }
}

pub struct FileProvider {
    files: Vec<SearchItem>,
}

impl FileProvider {
    pub fn from_files(files: Vec<SearchItem>) -> Self {
        Self { files }
    }

    pub fn deterministic_fixture() -> Self {
        Self {
            files: vec![
                SearchItem::new(
                    "file-report",
                    "file",
                    "Q4_Report.xlsx",
                    "C:\\Users\\Admin\\Documents\\Q4_Report.xlsx",
                ),
                SearchItem::new(
                    "file-notes",
                    "file",
                    "Meeting Notes.txt",
                    "C:\\Users\\Admin\\Documents\\Meeting Notes.txt",
                ),
            ],
        }
    }
}

impl DiscoveryProvider for FileProvider {
    fn provider_name(&self) -> &'static str {
        "file"
    }

    fn discover(&self) -> Result<Vec<SearchItem>, ProviderError> {
        Ok(self.files.clone())
    }
}

pub struct StartMenuAppDiscoveryProvider {
    roots: Vec<PathBuf>,
}

impl Default for StartMenuAppDiscoveryProvider {
    fn default() -> Self {
        Self {
            roots: default_start_menu_roots(),
        }
    }
}

impl StartMenuAppDiscoveryProvider {
    pub fn with_roots(roots: Vec<PathBuf>) -> Self {
        Self { roots }
    }
}

impl DiscoveryProvider for StartMenuAppDiscoveryProvider {
    fn provider_name(&self) -> &'static str {
        "start-menu-apps"
    }

    fn discover(&self) -> Result<Vec<SearchItem>, ProviderError> {
        #[cfg(not(target_os = "windows"))]
        {
            let _ = &self.roots;
            Ok(Vec::new())
        }

        #[cfg(target_os = "windows")]
        {
            let mut items = Vec::new();
            for root in &self.roots {
                items.extend(discover_start_menu_root(root)?);
            }
            Ok(items)
        }
    }
}

pub struct FileSystemDiscoveryProvider {
    roots: Vec<PathBuf>,
    excluded_roots: Vec<PathBuf>,
    max_depth: usize,
}

impl FileSystemDiscoveryProvider {
    pub fn new(roots: Vec<PathBuf>, max_depth: usize, excluded_roots: Vec<PathBuf>) -> Self {
        Self {
            roots,
            excluded_roots,
            max_depth,
        }
    }
}

impl DiscoveryProvider for FileSystemDiscoveryProvider {
    fn provider_name(&self) -> &'static str {
        "filesystem"
    }

    fn discover(&self) -> Result<Vec<SearchItem>, ProviderError> {
        let mut out = Vec::new();
        let excluded = normalized_exclusion_roots(&self.excluded_roots);

        for root in &self.roots {
            if !root.exists() {
                continue;
            }

            for entry in walkdir::WalkDir::new(root)
                .max_depth(self.max_depth)
                .into_iter()
                .filter_map(Result::ok)
            {
                let path = entry.path();
                if is_path_under_any_excluded_root(path, &excluded) {
                    continue;
                }
                if path.is_dir() {
                    if path == root {
                        continue;
                    }

                    let folder_name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| path.to_string_lossy().to_string());

                    let id = format!("folder:{}", path.to_string_lossy());
                    out.push(SearchItem::new(
                        &id,
                        "folder",
                        &folder_name,
                        &path.to_string_lossy(),
                    ));
                    continue;
                }

                if !path.is_file() {
                    continue;
                }

                let file_name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.to_string_lossy().to_string());

                let id = format!("file:{}", path.to_string_lossy());
                out.push(SearchItem::new(
                    &id,
                    "file",
                    &file_name,
                    &path.to_string_lossy(),
                ));
            }
        }

        Ok(out)
    }
}

fn normalized_exclusion_roots(excluded_roots: &[PathBuf]) -> Vec<String> {
    excluded_roots
        .iter()
        .filter_map(|root| normalize_path_for_compare(root).filter(|v| !v.is_empty()))
        .collect()
}

fn is_path_under_any_excluded_root(path: &Path, excluded_roots: &[String]) -> bool {
    let Some(path_norm) = normalize_path_for_compare(path) else {
        return false;
    };
    excluded_roots.iter().any(|root| {
        path_norm == *root
            || (path_norm.starts_with(root) && path_norm[root.len()..].starts_with('\\'))
    })
}

fn normalize_path_for_compare(path: &Path) -> Option<String> {
    let mut value = path.to_string_lossy().replace('/', "\\");
    while value.ends_with('\\') {
        value.pop();
    }
    let value = value.trim().to_ascii_lowercase();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

#[cfg(target_os = "windows")]
fn default_start_menu_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Ok(program_data) = std::env::var("ProgramData") {
        roots.push(
            PathBuf::from(program_data)
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs"),
        );
    }

    if let Ok(app_data) = std::env::var("APPDATA") {
        roots.push(
            PathBuf::from(app_data)
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs"),
        );
    }

    roots
}

#[cfg(not(target_os = "windows"))]
fn default_start_menu_roots() -> Vec<PathBuf> {
    Vec::new()
}

#[cfg(target_os = "windows")]
fn discover_start_menu_root(root: &Path) -> Result<Vec<SearchItem>, ProviderError> {
    let mut items = Vec::new();

    if !root.exists() {
        return Ok(items);
    }

    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();

        if ext != "lnk" && ext != "exe" {
            continue;
        }

        let title = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());
        let id = format!("app:{}", path.to_string_lossy());

        items.push(SearchItem::new(&id, "app", &title, &path.to_string_lossy()));
    }

    Ok(items)
}
