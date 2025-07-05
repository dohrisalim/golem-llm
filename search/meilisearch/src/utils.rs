use crate::bindings::exports::golem::search_meilisearch::core::{
    Schema, Doc, SearchQuery, SearchResults
};
use crate::bindings::golem::search_meilisearch::types::{SchemaField, FieldType, SearchHit};
use serde_json::Value;

pub fn schema_to_meilisearch_settings(schema: &Schema) -> Value {
    let mut searchable_attributes = Vec::new();
    let mut filterable_attributes = Vec::new();
    let mut sortable_attributes = Vec::new();
    
    for field in &schema.fields {
        if field.index {
            searchable_attributes.push(&field.name);
        }
        if field.facet {
            filterable_attributes.push(&field.name);
        }
        if field.sort {
            sortable_attributes.push(&field.name);
        }
    }
    
    let mut settings = serde_json::json!({
        "searchableAttributes": searchable_attributes,
        "filterableAttributes": filterable_attributes,
        "sortableAttributes": sortable_attributes
    });
    
    if let Some(primary_key) = &schema.primary_key {
        settings["primaryKey"] = serde_json::json!(primary_key);
    }
    
    settings
}

pub fn meilisearch_settings_to_schema(settings: &Value) -> Schema {
    let mut fields = Vec::new();
    
    let empty_vec = Vec::new();
    let searchable = settings.get("searchableAttributes")
        .and_then(|s| s.as_array())
        .unwrap_or(&empty_vec);
    
    let filterable = settings.get("filterableAttributes")
        .and_then(|f| f.as_array())
        .unwrap_or(&empty_vec);
    
    let sortable = settings.get("sortableAttributes")
        .and_then(|s| s.as_array())
        .unwrap_or(&empty_vec);
    
    // Collect all unique field names
    let mut field_names = std::collections::HashSet::new();
    for attr in searchable.iter().chain(filterable.iter()).chain(sortable.iter()) {
        if let Some(name) = attr.as_str() {
            field_names.insert(name);
        }
    }
    
    for name in field_names {
        let is_searchable = searchable.iter().any(|a| a.as_str() == Some(name));
        let is_filterable = filterable.iter().any(|a| a.as_str() == Some(name));
        let is_sortable = sortable.iter().any(|a| a.as_str() == Some(name));
        
        fields.push(SchemaField {
            name: name.to_string(),
            type_: FieldType::Text, // Meilisearch is primarily text-based
            required: false,
            facet: is_filterable,
            sort: is_sortable,
            index: is_searchable,
        });
    }
    
    let primary_key = settings.get("primaryKey")
        .and_then(|pk| pk.as_str())
        .map(|s| s.to_string());
    
    Schema {
        fields,
        primary_key,
    }
}

pub fn search_query_to_meilisearch(query: &SearchQuery) -> Value {
    let mut meilisearch_query = serde_json::json!({
        "q": query.q.as_deref().unwrap_or("")
    });
    
    // Handle pagination
    if let Some(page) = query.page {
        let per_page = query.per_page.unwrap_or(20);
        let offset = (page.saturating_sub(1)) * per_page;
        meilisearch_query["offset"] = serde_json::json!(offset);
        meilisearch_query["limit"] = serde_json::json!(per_page);
    } else if let Some(offset) = query.offset {
        meilisearch_query["offset"] = serde_json::json!(offset);
        if let Some(per_page) = query.per_page {
            meilisearch_query["limit"] = serde_json::json!(per_page);
        }
    }
    
    // Handle filters
    if !query.filters.is_empty() {
        meilisearch_query["filter"] = serde_json::json!(query.filters);
    }
    
    // Handle sorting
    if !query.sort.is_empty() {
        let sort_fields: Vec<String> = query.sort.iter().map(|s| {
            if s.starts_with('-') {
                format!("{}:desc", &s[1..])
            } else {
                format!("{}:asc", s)
            }
        }).collect();
        meilisearch_query["sort"] = serde_json::json!(sort_fields);
    }
    
    // Handle facets
    if !query.facets.is_empty() {
        meilisearch_query["facets"] = serde_json::json!(query.facets);
    }
    
    // Handle highlighting
    if let Some(highlight_config) = &query.highlight {
        let mut attributes_to_highlight = Vec::new();
        for field in &highlight_config.fields {
            attributes_to_highlight.push(format!("{}:*", field));
        }
        meilisearch_query["attributesToHighlight"] = serde_json::json!(attributes_to_highlight);
        
        if let Some(pre_tag) = &highlight_config.pre_tag {
            meilisearch_query["highlightPreTag"] = serde_json::json!(pre_tag);
        }
        if let Some(post_tag) = &highlight_config.post_tag {
            meilisearch_query["highlightPostTag"] = serde_json::json!(post_tag);
        }
    }
    
    meilisearch_query
}
