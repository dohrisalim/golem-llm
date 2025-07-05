use thiserror::Error;
use crate::bindings::golem::search_algolia::types::SearchError as WitSearchError;

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
}

impl From<SearchError> for WitSearchError {
    fn from(error: SearchError) -> Self {
        match error {
            SearchError::IndexNotFound(_) => WitSearchError::IndexNotFound,
            SearchError::InvalidQuery(msg) => WitSearchError::InvalidQuery(msg),
            SearchError::Unsupported(_msg) => WitSearchError::Unsupported,
            SearchError::Internal(msg) => WitSearchError::Internal(msg),
            SearchError::Timeout => WitSearchError::Timeout,
            SearchError::RateLimited => WitSearchError::RateLimited,
        }
    }
} 