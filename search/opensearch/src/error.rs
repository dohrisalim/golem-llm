use thiserror::Error;
use crate::bindings::golem::search_opensearch::types::SearchError as WitSearchError;

#[derive(Error, Debug)]
pub enum SearchError {
    #[error("Index not found: {0}")]
    IndexNotFound(String),
    #[error("Invalid query: {0}")]
    InvalidQuery(String),
    #[error("Unsupported operation: {0}")]
    Unsupported(String),
    #[error("Internal error: {0}")]
    Internal(String),
    #[error("Timeout occurred")]
    Timeout,
    #[error("Rate limited")]
    RateLimited,
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
}

impl From<SearchError> for WitSearchError {
    fn from(err: SearchError) -> Self {
        match err {
            SearchError::IndexNotFound(_) => WitSearchError::IndexNotFound,
            SearchError::InvalidQuery(msg) => WitSearchError::InvalidQuery(msg),
            SearchError::Unsupported(msg) => WitSearchError::Unsupported,
            SearchError::Internal(msg) => WitSearchError::Internal(msg),
            SearchError::Timeout => WitSearchError::Timeout,
            SearchError::RateLimited => WitSearchError::RateLimited,
            SearchError::Json(e) => WitSearchError::Internal(e.to_string()),
        }
    }
} 