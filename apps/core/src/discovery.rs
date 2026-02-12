use std::fmt::{Display, Formatter};

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
