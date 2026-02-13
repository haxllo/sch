use rusqlite::Connection;

use crate::action_executor::{launch_path, LaunchError};
use crate::config::{validate, Config};
use crate::contract::{CoreRequest, CoreResponse, LaunchResponse, SearchResponse};
use crate::discovery::{
    DiscoveryProvider, FileSystemDiscoveryProvider, ProviderError, StartMenuAppDiscoveryProvider,
};
use crate::index_store::{self, StoreError};
use crate::model::SearchItem;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::time::Instant;

#[derive(Debug)]
pub enum ServiceError {
    Config(String),
    Store(StoreError),
    Provider(ProviderError),
    Launch(LaunchError),
    InvalidRequest(String),
    ItemNotFound(String),
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Config(error) => write!(f, "config error: {error}"),
            Self::Store(error) => write!(f, "store error: {error}"),
            Self::Provider(error) => write!(f, "provider error: {error}"),
            Self::Launch(error) => write!(f, "launch error: {error}"),
            Self::InvalidRequest(error) => write!(f, "invalid request: {error}"),
            Self::ItemNotFound(id) => write!(f, "item not found: {id}"),
        }
    }
}

impl std::error::Error for ServiceError {}

impl From<StoreError> for ServiceError {
    fn from(value: StoreError) -> Self {
        Self::Store(value)
    }
}

impl From<LaunchError> for ServiceError {
    fn from(value: LaunchError) -> Self {
        Self::Launch(value)
    }
}

impl From<ProviderError> for ServiceError {
    fn from(value: ProviderError) -> Self {
        Self::Provider(value)
    }
}

pub enum LaunchTarget<'a> {
    Id(&'a str),
    Path(&'a str),
}

pub struct CoreService {
    config: Config,
    db: Connection,
    providers: Vec<Box<dyn DiscoveryProvider>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderRefreshReport {
    pub provider: String,
    pub discovered: usize,
    pub upserted: usize,
    pub removed: usize,
    pub elapsed_ms: u128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexRefreshReport {
    pub indexed_total: usize,
    pub discovered_total: usize,
    pub upserted_total: usize,
    pub removed_total: usize,
    pub providers: Vec<ProviderRefreshReport>,
}

impl CoreService {
    pub fn new(config: Config) -> Result<Self, ServiceError> {
        validate(&config).map_err(ServiceError::Config)?;
        let db = index_store::open_from_config(&config)?;
        Ok(Self {
            config,
            db,
            providers: Vec::new(),
        })
    }

    pub fn with_connection(config: Config, db: Connection) -> Result<Self, ServiceError> {
        validate(&config).map_err(ServiceError::Config)?;
        Ok(Self {
            config,
            db,
            providers: Vec::new(),
        })
    }

    pub fn with_providers(mut self, providers: Vec<Box<dyn DiscoveryProvider>>) -> Self {
        self.providers = providers;
        self
    }

    pub fn with_runtime_providers(mut self) -> Self {
        let mut providers: Vec<Box<dyn DiscoveryProvider>> = Vec::new();
        providers.push(Box::new(StartMenuAppDiscoveryProvider::default()));
        if !self.config.discovery_roots.is_empty() {
            providers.push(Box::new(FileSystemDiscoveryProvider::new(
                self.config.discovery_roots.clone(),
                5,
                self.config.discovery_exclude_roots.clone(),
            )));
        }
        self.providers = providers;
        self
    }

    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchItem>, ServiceError> {
        let all = index_store::list_items(&self.db)?;
        let mut valid = Vec::with_capacity(all.len());
        let mut stale_ids = Vec::new();

        for item in all {
            if is_stale_index_entry(&item) {
                stale_ids.push(item.id.clone());
            } else {
                valid.push(item);
            }
        }

        for stale_id in stale_ids {
            index_store::delete_item(&self.db, &stale_id)?;
        }

        let effective_limit = if limit == 0 {
            self.config.max_results as usize
        } else {
            limit.min(self.config.max_results as usize)
        };

        Ok(crate::search::search(&valid, query, effective_limit))
    }

    pub fn launch(&self, target: LaunchTarget<'_>) -> Result<(), ServiceError> {
        match target {
            LaunchTarget::Path(path) => launch_path(path).map_err(ServiceError::from),
            LaunchTarget::Id(id) => {
                let item = index_store::get_item(&self.db, id)?
                    .ok_or_else(|| ServiceError::ItemNotFound(id.to_string()))?;
                match launch_path(&item.path) {
                    Ok(()) => Ok(()),
                    Err(error @ LaunchError::MissingPath(_)) => {
                        index_store::delete_item(&self.db, &item.id)?;
                        Err(ServiceError::from(error))
                    }
                    Err(error) => Err(ServiceError::from(error)),
                }
            }
        }
    }

    pub fn rebuild_index(&self) -> Result<usize, ServiceError> {
        let report = self.rebuild_index_with_report()?;
        Ok(report.indexed_total)
    }

    pub fn rebuild_index_with_report(&self) -> Result<IndexRefreshReport, ServiceError> {
        if self.providers.is_empty() {
            return Ok(IndexRefreshReport {
                indexed_total: index_store::list_items(&self.db)?.len(),
                discovered_total: 0,
                upserted_total: 0,
                removed_total: 0,
                providers: Vec::new(),
            });
        }

        let mut existing_items = index_store::list_items(&self.db)?;
        let mut existing_by_id: HashMap<String, SearchItem> = existing_items
            .drain(..)
            .map(|item| (item.id.clone(), item))
            .collect();

        let mut discovered_total = 0_usize;
        let mut upserted_total = 0_usize;
        let mut removed_total = 0_usize;
        let mut provider_reports = Vec::with_capacity(self.providers.len());

        for provider in &self.providers {
            let started = Instant::now();
            let discovered = provider.discover()?;
            let discovered_count = discovered.len();
            discovered_total += discovered_count;

            let mut upserted = 0_usize;
            let mut discovered_ids = HashSet::with_capacity(discovered_count);

            for item in discovered {
                discovered_ids.insert(item.id.clone());
                let changed = existing_by_id
                    .get(&item.id)
                    .map(|previous| previous != &item)
                    .unwrap_or(true);
                if changed {
                    index_store::upsert_item(&self.db, &item)?;
                    upserted += 1;
                    upserted_total += 1;
                }
                existing_by_id.insert(item.id.clone(), item);
            }

            // Kind-based ownership is safe for current runtime provider composition:
            // start-menu apps own kind=app, filesystem owns kind=file/folder.
            let removable_ids: Vec<String> = existing_by_id
                .values()
                .filter(|item| provider_manages_kind(provider.provider_name(), &item.kind))
                .filter(|item| !discovered_ids.contains(&item.id))
                .map(|item| item.id.clone())
                .collect();

            for id in &removable_ids {
                index_store::delete_item(&self.db, id)?;
                existing_by_id.remove(id);
            }

            removed_total += removable_ids.len();
            provider_reports.push(ProviderRefreshReport {
                provider: provider.provider_name().to_string(),
                discovered: discovered_count,
                upserted,
                removed: removable_ids.len(),
                elapsed_ms: started.elapsed().as_millis(),
            });
        }

        let indexed_total = index_store::list_items(&self.db)?.len();
        Ok(IndexRefreshReport {
            indexed_total,
            discovered_total,
            upserted_total,
            removed_total,
            providers: provider_reports,
        })
    }

    pub fn rebuild_index_incremental(&self) -> Result<usize, ServiceError> {
        let report = self.rebuild_index_with_report()?;
        Ok(report.indexed_total)
    }

    pub fn upsert_item(&self, item: &SearchItem) -> Result<(), ServiceError> {
        index_store::upsert_item(&self.db, item)?;
        Ok(())
    }

    pub fn handle_command(&self, request: CoreRequest) -> Result<CoreResponse, ServiceError> {
        match request {
            CoreRequest::Search(search) => {
                let results = self.search(&search.query, search.limit.unwrap_or(0))?;
                Ok(CoreResponse::Search(SearchResponse {
                    results: results.into_iter().map(Into::into).collect(),
                }))
            }
            CoreRequest::Launch(launch) => {
                if let Some(id) = launch.id.as_deref() {
                    if !id.trim().is_empty() {
                        self.launch(LaunchTarget::Id(id))?;
                        return Ok(CoreResponse::Launch(LaunchResponse { launched: true }));
                    }
                }

                if let Some(path) = launch.path.as_deref() {
                    if !path.trim().is_empty() {
                        self.launch(LaunchTarget::Path(path))?;
                        return Ok(CoreResponse::Launch(LaunchResponse { launched: true }));
                    }
                }

                Err(ServiceError::InvalidRequest(
                    "launch requires non-empty id or path".into(),
                ))
            }
        }
    }
}

fn is_stale_index_entry(item: &SearchItem) -> bool {
    if !(item.kind.eq_ignore_ascii_case("app")
        || item.kind.eq_ignore_ascii_case("file")
        || item.kind.eq_ignore_ascii_case("folder"))
    {
        return false;
    }

    let path = item.path.trim();
    if path.is_empty() {
        return false;
    }
    if path.contains("://") {
        return false;
    }
    if !looks_like_filesystem_path(path) {
        return false;
    }

    !Path::new(path).exists()
}

fn looks_like_filesystem_path(path: &str) -> bool {
    if path.starts_with('/') || path.starts_with('\\') {
        return true;
    }

    let bytes = path.as_bytes();
    bytes.len() >= 3 && bytes[1] == b':' && (bytes[2] == b'\\' || bytes[2] == b'/')
}

fn provider_manages_kind(provider_name: &str, kind: &str) -> bool {
    let kind = kind.to_ascii_lowercase();
    match provider_name {
        "start-menu-apps" | "app" => kind == "app",
        "filesystem" | "file" => kind == "file" || kind == "folder",
        _ => false,
    }
}
