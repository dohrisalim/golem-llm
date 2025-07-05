mod bindings;
mod client;
mod config;
mod error;
mod utils;

use bindings::exports::golem::search_meilisearch::core::Guest;
use bindings::golem::search_meilisearch::types::{
    IndexName, DocumentId, Doc, Schema, SearchQuery, SearchResults, SearchHit, SearchError
};
use client::MeilisearchClient;
use config::SearchConfig;
use error::SearchError as InternalError;
use utils::*;

struct Component;

bindings::export!(Component with_types_in bindings);

impl Guest for Component {
    // Index lifecycle
    fn create_index(name: IndexName, schema: Option<Schema>) -> Result<(), SearchError> {
        let client = MeilisearchClient::new().map_err(|e| SearchError::Internal(e.to_string()))?;
        client.create_index(name, schema).map_err(|e| e.into())
    }

    fn delete_index(name: IndexName) -> Result<(), SearchError> {
        let client = MeilisearchClient::new().map_err(|e| SearchError::Internal(e.to_string()))?;
        client.delete_index(name).map_err(|e| e.into())
    }

    fn list_indexes() -> Result<Vec<IndexName>, SearchError> {
        let client = MeilisearchClient::new().map_err(|e| SearchError::Internal(e.to_string()))?;
        client.list_indexes().map_err(|e| e.into())
    }

    // Document operations
    fn upsert(index: IndexName, doc: Doc) -> Result<(), SearchError> {
        let client = MeilisearchClient::new().map_err(|e| SearchError::Internal(e.to_string()))?;
        client.upsert(index, doc).map_err(|e| e.into())
    }

    fn upsert_many(index: IndexName, docs: Vec<Doc>) -> Result<(), SearchError> {
        let client = MeilisearchClient::new().map_err(|e| SearchError::Internal(e.to_string()))?;
        client.upsert_many(index, docs).map_err(|e| e.into())
    }

    fn delete(index: IndexName, id: DocumentId) -> Result<(), SearchError> {
        let client = MeilisearchClient::new().map_err(|e| SearchError::Internal(e.to_string()))?;
        client.delete(index, id).map_err(|e| e.into())
    }

    fn delete_many(index: IndexName, ids: Vec<DocumentId>) -> Result<(), SearchError> {
        let client = MeilisearchClient::new().map_err(|e| SearchError::Internal(e.to_string()))?;
        client.delete_many(index, ids).map_err(|e| e.into())
    }

    fn get(index: IndexName, id: DocumentId) -> Result<Option<Doc>, SearchError> {
        let client = MeilisearchClient::new().map_err(|e| SearchError::Internal(e.to_string()))?;
        client.get(index, id).map_err(|e| e.into())
    }

    // Query
    fn search(index: IndexName, query: SearchQuery) -> Result<SearchResults, SearchError> {
        let client = MeilisearchClient::new().map_err(|e| SearchError::Internal(e.to_string()))?;
        client.search(index, query).map_err(|e| e.into())
    }

    // Schema inspection
    fn get_schema(index: IndexName) -> Result<Schema, SearchError> {
        let client = MeilisearchClient::new().map_err(|e| SearchError::Internal(e.to_string()))?;
        client.get_schema(index).map_err(|e| e.into())
    }

    fn update_schema(index: IndexName, schema: Schema) -> Result<(), SearchError> {
        let client = MeilisearchClient::new().map_err(|e| SearchError::Internal(e.to_string()))?;
        client.update_schema(index, schema).map_err(|e| e.into())
    }
}

