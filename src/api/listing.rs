//! Listing handlers for PipeWire objects
//!
//! Uses pw-cli for simple and reliable object listing.

use axum::{
    extract::{Path, State},
    Json,
};
use std::sync::Arc;

use crate::api_server::{ApiError, AppState};
use crate::pwcli;
use super::types::*;

/// Convert NodeTypeClassification to string for API response
fn classification_to_string(classification: pwcli::NodeTypeClassification) -> String {
    match classification {
        pwcli::NodeTypeClassification::Audio => "Audio".to_string(),
        pwcli::NodeTypeClassification::Midi => "Midi".to_string(),
        pwcli::NodeTypeClassification::Video => "Video".to_string(),
        pwcli::NodeTypeClassification::Link => "Link".to_string(),
        pwcli::NodeTypeClassification::Port => "Port".to_string(),
        pwcli::NodeTypeClassification::Client => "Client".to_string(),
        pwcli::NodeTypeClassification::Driver => "Driver".to_string(),
        pwcli::NodeTypeClassification::Other => "Other".to_string(),
        pwcli::NodeTypeClassification::Unknown => "Unknown".to_string(),
    }
}

/// Convert a pwcli::PwObject to our API PipeWireObject
fn to_api_object(obj: &pwcli::PwObject) -> PipeWireObject {
    // First check media.class
    let mut classification = pwcli::classify_media_class(obj.media_class());
    
    // If Unknown, check object_type for specific types that don't have media.class
    if classification == pwcli::NodeTypeClassification::Unknown {
        let simplified_type = pwcli::simplify_type(&obj.object_type);
        classification = match simplified_type {
            "link" => pwcli::NodeTypeClassification::Link,
            "port" => pwcli::NodeTypeClassification::Port,
            "client" => pwcli::NodeTypeClassification::Client,
            "module" => pwcli::NodeTypeClassification::Other,
            "factory" => pwcli::NodeTypeClassification::Other,
            _ => pwcli::NodeTypeClassification::Unknown,
        };
    }
    
    // Check for driver nodes (Dummy-Driver, Freewheel-Driver, etc.)
    if classification == pwcli::NodeTypeClassification::Unknown && pwcli::is_driver_node(obj) {
        classification = pwcli::NodeTypeClassification::Driver;
    }
    
    PipeWireObject {
        id: obj.id,
        name: obj.display_name(),
        object_type: pwcli::simplify_type(&obj.object_type).to_string(),
        media_class: Some(classification_to_string(classification)),
    }
}

/// List all PipeWire objects
pub async fn list_all(State(_state): State<Arc<AppState>>) -> Result<Json<ListResponse>, ApiError> {
    let objects = pwcli::list_all()
        .map_err(|e| ApiError::Internal(format!("Failed to list objects: {}", e)))?;
    
    let api_objects: Vec<PipeWireObject> = objects.iter()
        .map(to_api_object)
        .collect();
    
    Ok(Json(ListResponse { objects: api_objects }))
}



/// Get a single object by ID
pub async fn get_object_by_id(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> Result<Json<PipeWireObject>, ApiError> {
    // First try the cache
    if let Some(obj) = state.get_object_by_id(id) {
        return Ok(Json(to_api_object(&obj)));
    }
    
    // If not in cache, try to get it directly from pw-cli
    match pwcli::get_object(id) {
        Ok(Some(obj)) => Ok(Json(to_api_object(&obj))),
        Ok(None) => Err(ApiError::NotFound(format!("Object {} not found", id))),
        Err(e) => Err(ApiError::Internal(format!("Failed to get object: {}", e))),
    }
}

/// Refresh the object cache
pub async fn refresh_cache(State(state): State<Arc<AppState>>) -> Result<Json<serde_json::Value>, ApiError> {
    if let Err(e) = state.refresh_object_cache() {
        return Err(ApiError::Internal(format!("Failed to refresh cache: {}", e)));
    }
    let count = state.get_cached_objects().len();
    Ok(Json(serde_json::json!({
        "status": "ok",
        "message": "Cache refreshed",
        "object_count": count
    })))
}
