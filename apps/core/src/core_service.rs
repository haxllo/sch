use rusqlite::Connection;

use crate::action_executor::{launch_path, LaunchError};
use crate::config::{validate, Config};
use crate::contract::{CoreRequest, CoreResponse, LaunchResponse, SearchResponse};
use crate::discovery::{DiscoveryProvider, ProviderError};
use crate::index_store::{self, StoreError};
use crate::model::SearchItem;

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

    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchItem>, ServiceError> {
        let all = index_store::list_items(&self.db)?;
        let effective_limit = if limit == 0 {
            self.config.max_results as usize
        } else {
            limit.min(self.config.max_results as usize)
        };

        Ok(crate::search::search(&all, query, effective_limit))
    }

    pub fn launch(&self, target: LaunchTarget<'_>) -> Result<(), ServiceError> {
        match target {
            LaunchTarget::Path(path) => launch_path(path).map_err(ServiceError::from),
            LaunchTarget::Id(id) => {
                let item = index_store::get_item(&self.db, id)?
                    .ok_or_else(|| ServiceError::ItemNotFound(id.to_string()))?;
                launch_path(&item.path).map_err(ServiceError::from)
            }
        }
    }

    pub fn rebuild_index(&self) -> Result<usize, ServiceError> {
        if self.providers.is_empty() {
            return Ok(index_store::list_items(&self.db)?.len());
        }

        index_store::clear_items(&self.db)?;

        let mut inserted = 0_usize;
        for provider in &self.providers {
            let discovered = provider.discover()?;
            for item in discovered {
                index_store::upsert_item(&self.db, &item)?;
                inserted += 1;
            }
        }

        Ok(inserted)
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
