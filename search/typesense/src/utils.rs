use crate::bindings::exports::golem::search_typesense::core::{
    Schema, Doc, SearchQuery, SearchResults
};
use crate::bindings::golem::search_typesense::types::{SchemaField, FieldType, SearchHit};
use serde_json::Value;

/// Convert WIT Schema to Typesense collection
pub fn schema_to_typesense_collection(schema: &Schema) -> Value {
    let mut fields = Vec::new();
    
    for field in &schema.fields {
        let field_type = match field.type_ {
            FieldType::Text => "string",
            FieldType::Keyword => "string",
            FieldType::Integer => "int32",
            FieldType::Float => "float",
            FieldType::Boolean => "bool",
            FieldType::Date => "string", // Typesense doesn't have native date type
            FieldType::GeoPoint => "geopoint",
        };
        
        let mut typesense_field = serde_json::json!({
            "name": field.name,
            "type": field_type,
            "facet": field.facet,
            "index": field.index,
            "sort": field.sort
        });
        
        if field.required {
            typesense_field["optional"] = serde_json::json!(false);
        }
        
        fields.push(typesense_field);
    }
    
    serde_json::json!({
        "fields": fields,
        "default_sorting_field": schema.primary_key.as_deref().unwrap_or("id")
    })
}

/// Convert Typesense collection to WIT Schema
pub fn typesense_collection_to_schema(collection: &Value) -> Schema {
    let mut fields = Vec::new();
    
    if let Some(typesense_fields) = collection.get("fields").and_then(|f| f.as_array()) {
        for field_def in typesense_fields {
            if let Some(field_name) = field_def.get("name").and_then(|n| n.as_str()) {
                let field_type = match field_def.get("type").and_then(|t| t.as_str()) {
                    Some("string") => FieldType::Text,
                    Some("int32") | Some("int64") => FieldType::Integer,
                    Some("float") => FieldType::Float,
                    Some("bool") => FieldType::Boolean,
                    Some("geopoint") => FieldType::GeoPoint,
                    _ => FieldType::Text,
                };
                
                let required = field_def.get("optional")
                    .and_then(|o| o.as_bool())
                    .map(|opt| !opt)
                    .unwrap_or(false);
                
                let facet = field_def.get("facet")
                    .and_then(|f| f.as_bool())
                    .unwrap_or(false);
                
                let sort = field_def.get("sort")
                    .and_then(|s| s.as_bool())
                    .unwrap_or(true);
                
                let index = field_def.get("index")
                    .and_then(|i| i.as_bool())
                    .unwrap_or(true);
                
                fields.push(SchemaField {
                    name: field_name.to_string(),
                    type_: field_type,
                    required,
                    facet,
                    sort,
                    index,
                });
            }
        }
    }
    
    let primary_key = collection.get("default_sorting_field")
        .and_then(|pk| pk.as_str())
        .map(|s| s.to_string());
    
    Schema {
        fields,
        primary_key,
    }
}

/// Convert search query to Typesense query
pub fn search_query_to_typesense(query: &SearchQuery) -> Value {
    let mut typesense_query = serde_json::json!({
        "q": query.q.as_deref().unwrap_or("*"),
        "query_by": "title,description" // Default search fields
    });
    
    // Handle pagination
    if let Some(page) = query.page {
        let per_page = query.per_page.unwrap_or(10);
        typesense_query["page"] = serde_json::json!(page);
        typesense_query["per_page"] = serde_json::json!(per_page);
    } else if let Some(offset) = query.offset {
        let per_page = query.per_page.unwrap_or(10);
        let page = (offset / per_page) + 1;
        typesense_query["page"] = serde_json::json!(page);
        typesense_query["per_page"] = serde_json::json!(per_page);
    }
    
    // Handle filters
    if !query.filters.is_empty() {
        typesense_query["filter_by"] = serde_json::json!(query.filters.join(" && "));
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
        typesense_query["sort_by"] = serde_json::json!(sort_fields.join(","));
    }
    
    // Handle facets
    if !query.facets.is_empty() {
        typesense_query["facet_by"] = serde_json::json!(query.facets.join(","));
    }
    
    // Handle highlighting
    if let Some(highlight_config) = &query.highlight {
        typesense_query["highlight_full_fields"] = serde_json::json!(highlight_config.fields.join(","));
        if let Some(pre_tag) = &highlight_config.pre_tag {
            typesense_query["highlight_start_tag"] = serde_json::json!(pre_tag);
        }
        if let Some(post_tag) = &highlight_config.post_tag {
            typesense_query["highlight_end_tag"] = serde_json::json!(post_tag);
        }
    }
    
    typesense_query
} 