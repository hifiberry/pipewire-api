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

/// List all nodes
pub async fn list_nodes(State(_state): State<Arc<AppState>>) -> Result<Json<ListResponse>, ApiError> {
    let objects = pwcli::list_nodes()
        .map_err(|e| ApiError::Internal(format!("Failed to list nodes: {}", e)))?;
    
    let api_objects: Vec<PipeWireObject> = objects.iter()
        .map(to_api_object)
        .collect();
    
    Ok(Json(ListResponse { objects: api_objects }))
}

/// List all devices
pub async fn list_devices(State(_state): State<Arc<AppState>>) -> Result<Json<ListResponse>, ApiError> {
    let objects = pwcli::list_devices()
        .map_err(|e| ApiError::Internal(format!("Failed to list devices: {}", e)))?;
    
    let api_objects: Vec<PipeWireObject> = objects.iter()
        .map(to_api_object)
        .collect();
    
    Ok(Json(ListResponse { objects: api_objects }))
}

/// List all ports
pub async fn list_ports(State(_state): State<Arc<AppState>>) -> Result<Json<ListResponse>, ApiError> {
    let objects = pwcli::list_ports()
        .map_err(|e| ApiError::Internal(format!("Failed to list ports: {}", e)))?;
    
    let api_objects: Vec<PipeWireObject> = objects.iter()
        .map(to_api_object)
        .collect();
    
    Ok(Json(ListResponse { objects: api_objects }))
}

/// List all modules
pub async fn list_modules(State(_state): State<Arc<AppState>>) -> Result<Json<ListResponse>, ApiError> {
    let objects = pwcli::list_modules()
        .map_err(|e| ApiError::Internal(format!("Failed to list modules: {}", e)))?;
    
    let api_objects: Vec<PipeWireObject> = objects.iter()
        .map(to_api_object)
        .collect();
    
    Ok(Json(ListResponse { objects: api_objects }))
}

/// List all factories
pub async fn list_factories(State(_state): State<Arc<AppState>>) -> Result<Json<ListResponse>, ApiError> {
    let objects = pwcli::list_factories()
        .map_err(|e| ApiError::Internal(format!("Failed to list factories: {}", e)))?;
    
    let api_objects: Vec<PipeWireObject> = objects.iter()
        .map(to_api_object)
        .collect();
    
    Ok(Json(ListResponse { objects: api_objects }))
}

/// List all clients
pub async fn list_clients(State(_state): State<Arc<AppState>>) -> Result<Json<ListResponse>, ApiError> {
    let objects = pwcli::list_clients()
        .map_err(|e| ApiError::Internal(format!("Failed to list clients: {}", e)))?;
    
    let api_objects: Vec<PipeWireObject> = objects.iter()
        .map(to_api_object)
        .collect();
    
    Ok(Json(ListResponse { objects: api_objects }))
}

/// List all links
pub async fn list_links(State(_state): State<Arc<AppState>>) -> Result<Json<ListResponse>, ApiError> {
    let objects = pwcli::list_links()
        .map_err(|e| ApiError::Internal(format!("Failed to list links: {}", e)))?;
    
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
