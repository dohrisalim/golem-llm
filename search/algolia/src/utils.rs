use crate::bindings::exports::golem::search_algolia::core::{
    Schema, Doc, SearchQuery, SearchResults
};
use crate::bindings::golem::search_algolia::types::{SchemaField, FieldType, SearchHit};
use serde_json::Value;

/// Convert WIT Schema to Elasticsearch mapping
pub fn schema_to_elastic_mapping(schema: &Schema) -> Value {
    let mut properties = serde_json::json!({});
    
    for field in &schema.fields {
        let mut field_mapping = serde_json::json!({});
        
        let field_type = match field.type_ {
            FieldType::Text => "text",
            FieldType::Keyword => "keyword",
            FieldType::Integer => "integer",
            FieldType::Float => "float",
            FieldType::Boolean => "boolean",
            FieldType::Date => "date",
            FieldType::GeoPoint => "geo_point",
        };
        
        field_mapping["type"] = serde_json::json!(field_type);
        
        if field.index {
            field_mapping["index"] = serde_json::json!(true);
        }
        
        properties[field.name.clone()] = field_mapping;
    }
    
    let mut mapping = serde_json::json!({
        "properties": properties
    });
    
    mapping
}

/// Convert Elasticsearch mapping to WIT Schema
pub fn elastic_mapping_to_schema(mapping: &Value) -> Schema {
    let mut fields = Vec::new();
    
    if let Some(properties) = mapping.get("mappings")
        .and_then(|m| m.get("properties"))
        .and_then(|p| p.as_object()) {
        
        for (name, field_def) in properties {
            if let Some(field_obj) = field_def.as_object() {
                let field_type = match field_obj.get("type").and_then(|t| t.as_str()) {
                    Some("text") => FieldType::Text,
                    Some("keyword") => FieldType::Keyword,
                    Some("integer") | Some("long") => FieldType::Integer,
                    Some("float") | Some("double") => FieldType::Float,
                    Some("boolean") => FieldType::Boolean,
                    Some("date") => FieldType::Date,
                    Some("geo_point") => FieldType::GeoPoint,
                    _ => FieldType::Text, // Default fallback
                };
                
                let index = field_obj.get("index")
                    .and_then(|i| i.as_bool())
                    .unwrap_or(true);
                
                fields.push(SchemaField {
                    name: name.clone(),
                    type_: field_type,
                    required: false, // Elasticsearch doesn't have required fields by default
                    facet: false,    // Would need to check if aggregatable
                    sort: true,      // Most fields are sortable
                    index,
                });
            }
        }
    }
    
    Schema {
        fields,
        primary_key: Some("_id".to_string()),
    }
}

/// Convert search query to Elasticsearch query DSL
pub fn search_query_to_elastic(query: &SearchQuery) -> Value {
    let mut elastic_query = serde_json::json!({});
    let mut query_body = serde_json::json!({});
    
    // Handle the main query
    if let Some(q) = &query.q {
        if !q.is_empty() {
            let mut match_query = serde_json::json!({
                "query": q.clone()
            });
            query_body["multi_match"] = match_query;
        }
    } else {
        query_body["match_all"] = serde_json::json!(true);
    }
    
    elastic_query["query"] = query_body;
    
    // Handle pagination
    if let Some(page) = query.page {
        if let Some(per_page) = query.per_page {
            let from = (page.saturating_sub(1)) * per_page;
            elastic_query["from"] = serde_json::json!(from);
            elastic_query["size"] = serde_json::json!(per_page);
        }
    } else if let Some(offset) = query.offset {
        elastic_query["from"] = serde_json::json!(offset);
        if let Some(per_page) = query.per_page {
            elastic_query["size"] = serde_json::json!(per_page);
        }
    }
    
    // Handle sorting
    if !query.sort.is_empty() {
        let sort_array: Vec<Value> = query.sort.iter().map(|s| {
            if s.starts_with('-') {
                let field = &s[1..];
                let mut sort_obj = serde_json::json!({});
                sort_obj[field] = serde_json::json!("desc");
                sort_obj
            } else {
                let mut sort_obj = serde_json::json!({});
                sort_obj[s.clone()] = serde_json::json!("asc");
                sort_obj
            }
        }).collect();
        elastic_query["sort"] = serde_json::json!(sort_array);
    }
    
    // Handle highlighting
    if let Some(highlight_config) = &query.highlight {
        let mut highlight = serde_json::json!({});
        let mut fields = serde_json::json!({});
        
        for field in &highlight_config.fields {
            let mut field_config = serde_json::json!({});
            if let Some(max_len) = highlight_config.max_length {
                field_config["fragment_size"] = serde_json::json!(max_len);
            }
            fields[field.clone()] = field_config;
        }
        
        highlight["fields"] = fields;
        
        if let Some(pre_tag) = &highlight_config.pre_tag {
            highlight["pre_tags"] = serde_json::json!(vec![pre_tag.clone()]);
        }
        if let Some(post_tag) = &highlight_config.post_tag {
            highlight["post_tags"] = serde_json::json!(vec![post_tag.clone()]);
        }
        
        elastic_query["highlight"] = highlight;
    }
    
    elastic_query
}

pub fn schema_to_algolia_settings(schema: &Schema) -> Value {
    let mut settings = serde_json::json!({
        "attributesToIndex": [],
        "attributesForFaceting": [],
        "searchableAttributes": []
    });
    
    for field in &schema.fields {
        if field.index {
            settings["searchableAttributes"].as_array_mut().unwrap().push(serde_json::json!(field.name));
        }
        
        if field.facet {
            settings["attributesForFaceting"].as_array_mut().unwrap().push(serde_json::json!(field.name));
        }
    }
    
    settings
}

pub fn algolia_settings_to_schema(settings: &Value) -> Schema {
    let mut fields = Vec::new();
    
    if let Some(searchable) = settings.get("searchableAttributes").and_then(|s| s.as_array()) {
        for field_name in searchable {
            if let Some(name) = field_name.as_str() {
                fields.push(SchemaField {
                    name: name.to_string(),
                    type_: FieldType::Text, // Algolia is primarily text-based
                    required: false,
                    facet: false,
                    sort: true,
                    index: true,
                });
            }
        }
    }
    
    Schema {
        fields,
        primary_key: Some("objectID".to_string()), // Algolia uses objectID
    }
}

pub fn search_query_to_algolia(query: &SearchQuery) -> Value {
    let mut algolia_query = serde_json::json!({
        "query": query.q.as_deref().unwrap_or(""),
        "hitsPerPage": query.per_page.unwrap_or(10)
    });
    
    // Handle pagination
    if let Some(page) = query.page {
        algolia_query["page"] = serde_json::json!(page - 1); // Algolia uses 0-based pages
    } else if let Some(offset) = query.offset {
        let per_page = query.per_page.unwrap_or(10);
        algolia_query["page"] = serde_json::json!(offset / per_page);
    }
    
    // Handle filters (simplified - Algolia has complex filter syntax)
    if !query.filters.is_empty() {
        let filter_string = query.filters.join(" AND ");
        algolia_query["filters"] = serde_json::json!(filter_string);
    }
    
    // Handle facets (simplified)
    if !query.facets.is_empty() {
        algolia_query["facets"] = serde_json::json!(query.facets);
    }
    
    // Handle highlighting
    if let Some(highlight_config) = &query.highlight {
        algolia_query["highlightPreTag"] = serde_json::json!(highlight_config.pre_tag.as_deref().unwrap_or("<em>"));
        algolia_query["highlightPostTag"] = serde_json::json!(highlight_config.post_tag.as_deref().unwrap_or("</em>"));
        algolia_query["attributesToHighlight"] = serde_json::json!(highlight_config.fields);
    }
    
    algolia_query
} 