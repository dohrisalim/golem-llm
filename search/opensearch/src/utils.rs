use crate::bindings::golem::search_opensearch::types::{
    Schema, SchemaField, FieldType, Doc, SearchQuery, SearchResults, SearchHit
};
use serde_json::Value;

/// Convert WIT Schema to Elasticsearch mapping
pub fn schema_to_elastic_mapping(schema: &Schema) -> Value {
    let mut mapping = serde_json::json!({
        "properties": {}
    });
    
    if let Some(properties) = mapping.get_mut("properties") {
        for field in &schema.fields {
            let field_mapping = match field.type_ {
                FieldType::Text => serde_json::json!({
                    "type": "text",
                    "index": field.index
                }),
                FieldType::Keyword => serde_json::json!({
                    "type": "keyword",
                    "index": field.index
                }),
                FieldType::Integer => serde_json::json!({
                    "type": "integer"
                }),
                FieldType::Float => serde_json::json!({
                    "type": "float"
                }),
                FieldType::Boolean => serde_json::json!({
                    "type": "boolean"
                }),
                FieldType::Date => serde_json::json!({
                    "type": "date"
                }),
                FieldType::GeoPoint => serde_json::json!({
                    "type": "geo_point"
                }),
            };
            
            properties.as_object_mut().unwrap().insert(field.name.clone(), field_mapping);
        }
    }
    
    mapping
}

/// Convert Elasticsearch mapping to WIT Schema
pub fn elastic_mapping_to_schema(mapping: &Value) -> Schema {
    let mut fields = Vec::new();
    
    if let Some(properties) = mapping.get("properties").and_then(|p| p.as_object()) {
        for (field_name, field_def) in properties {
            if let Some(field_type_str) = field_def.get("type").and_then(|t| t.as_str()) {
                let field_type = match field_type_str {
                    "text" => FieldType::Text,
                    "keyword" => FieldType::Keyword,
                    "integer" | "long" => FieldType::Integer,
                    "float" | "double" => FieldType::Float,
                    "boolean" => FieldType::Boolean,
                    "date" => FieldType::Date,
                    "geo_point" => FieldType::GeoPoint,
                    _ => FieldType::Text, // Default fallback
                };
                
                let index = field_def.get("index")
                    .and_then(|i| i.as_bool())
                    .unwrap_or(true);
                
                fields.push(SchemaField {
                    name: field_name.clone(),
                    type_: field_type,
                    required: false, // OpenSearch doesn't have required fields in the same way
                    facet: false,    // Would need to check aggregatable
                    sort: true,      // Most fields are sortable by default
                    index,
                });
            }
        }
    }
    
    Schema {
        fields,
        primary_key: None, // OpenSearch uses _id by default
    }
}

/// Convert search query to Elasticsearch query DSL
pub fn search_query_to_elastic(query: &SearchQuery) -> Value {
    let mut elastic_query = serde_json::json!({
        "query": {
            "match_all": {}
        }
    });
    
    // Handle text query
    if let Some(q) = &query.q {
        elastic_query["query"] = serde_json::json!({
            "multi_match": {
                "query": q,
                "type": "best_fields"
            }
        });
    }
    
    // Handle filters (simplified)
    if !query.filters.is_empty() {
        // For now, treat filters as additional query terms
        // In practice, these would be parsed and converted to proper OpenSearch filters
        let filter_queries: Vec<Value> = query.filters.iter().map(|f| {
            serde_json::json!({
                "query_string": {
                    "query": f
                }
            })
        }).collect();
        
        elastic_query["query"] = serde_json::json!({
            "bool": {
                "must": [elastic_query["query"]],
                "filter": filter_queries
            }
        });
    }
    
    // Handle sorting
    if !query.sort.is_empty() {
        let sort_fields: Vec<Value> = query.sort.iter().map(|s| {
            if s.starts_with('-') {
                serde_json::json!({ &s[1..]: { "order": "desc" } })
            } else {
                serde_json::json!({ s: { "order": "asc" } })
            }
        }).collect();
        elastic_query["sort"] = serde_json::json!(sort_fields);
    }
    
    // Handle pagination
    if let Some(page) = query.page {
        let per_page = query.per_page.unwrap_or(10);
        elastic_query["from"] = serde_json::json!((page - 1) * per_page);
        elastic_query["size"] = serde_json::json!(per_page);
    } else if let Some(offset) = query.offset {
        elastic_query["from"] = serde_json::json!(offset);
        if let Some(per_page) = query.per_page {
            elastic_query["size"] = serde_json::json!(per_page);
        }
    }
    
    // Handle highlighting
    if let Some(highlight_config) = &query.highlight {
        let mut highlight = serde_json::json!({
            "fields": {}
        });
        
        for field in &highlight_config.fields {
            highlight["fields"][field] = serde_json::json!({});
        }
        
        if let Some(pre_tag) = &highlight_config.pre_tag {
            highlight["pre_tags"] = serde_json::json!([pre_tag]);
        }
        
        if let Some(post_tag) = &highlight_config.post_tag {
            highlight["post_tags"] = serde_json::json!([post_tag]);
        }
        
        elastic_query["highlight"] = highlight;
    }
    
    elastic_query
} 