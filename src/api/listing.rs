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

/// Convert a pwcli::PwObject to our API PipeWireObject
fn to_api_object(obj: &pwcli::PwObject) -> PipeWireObject {
    PipeWireObject {
        id: obj.id,
        name: obj.display_name(),
        object_type: pwcli::simplify_type(&obj.object_type).to_string(),
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
