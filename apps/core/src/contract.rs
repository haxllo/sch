use serde::{Deserialize, Serialize};

use crate::model::SearchItem;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchRequest {
    pub query: String,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchResultDto {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchResponse {
    pub results: Vec<SearchResultDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LaunchRequest {
    pub id: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LaunchResponse {
    pub launched: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "payload")]
pub enum CoreRequest {
    Search(SearchRequest),
    Launch(LaunchRequest),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "payload")]
pub enum CoreResponse {
    Search(SearchResponse),
    Launch(LaunchResponse),
}

impl From<SearchItem> for SearchResultDto {
    fn from(value: SearchItem) -> Self {
        Self {
            id: value.id,
            kind: value.kind,
            title: value.title,
            path: value.path,
        }
    }
}
