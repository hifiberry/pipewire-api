//! Properties handlers for PipeWire objects
//!
//! Uses pw-cli for simple and reliable property listing.

use axum::{
    extract::{Path, State},
    Json,
};
use std::sync::Arc;

use crate::api_server::{ApiError, AppState};
use crate::pwcli;
use super::types::*;

/// Convert a pwcli::PwObject to PipeWireObjectWithProperties
fn to_object_with_properties(obj: &pwcli::PwObject) -> PipeWireObjectWithProperties {
    PipeWireObjectWithProperties {
        id: obj.id,
        name: obj.display_name(),
        object_type: pwcli::simplify_type(&obj.object_type).to_string(),
        properties: obj.properties.clone(),
        dynamic_properties: None,  // pw-cli doesn't provide dynamic properties
    }
}

/// List all PipeWire objects with their properties
/// GET /api/v1/properties
pub async fn list_all_properties(
    State(state): State<Arc<AppState>>,
) -> Result<Json<PropertiesResponse>, ApiError> {
    // Try to use cached objects first
    let cached = state.get_cached_objects();

    let objects = if !cached.is_empty() {
        cached
    } else {
        // Fall back to fresh query
        tokio::task::spawn_blocking(|| {
            pwcli::list_all()
        })
        .await
        .map_err(|e| ApiError::Internal(format!("Task join error: {}", e)))?
        .map_err(|e| ApiError::Internal(format!("Failed to list objects: {}", e)))?
    };

    let objects_with_props: Vec<PipeWireObjectWithProperties> = objects.iter()
        .map(to_object_with_properties)
        .collect();

    Ok(Json(PropertiesResponse { objects: objects_with_props }))
}

/// Get properties for a specific object by ID
/// GET /api/v1/properties/:id
pub async fn get_object_properties(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> Result<Json<PipeWireObjectWithProperties>, ApiError> {
    // Try cache first
    if let Some(obj) = state.get_object_by_id(id) {
        return Ok(Json(to_object_with_properties(&obj)));
    }

    // Fall back to fresh query
    let result = tokio::task::spawn_blocking(move || {
        pwcli::get_object(id)
    })
    .await
    .map_err(|e| ApiError::Internal(format!("Task join error: {}", e)))?;

    match result {
        Ok(Some(obj)) => Ok(Json(to_object_with_properties(&obj))),
        Ok(None) => Err(ApiError::NotFound(format!("Object {} not found", id))),
        Err(e) => Err(ApiError::Internal(format!("Failed to get object: {}", e))),
    }
}
