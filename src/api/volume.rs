//! Unified volume API using wpctl
//! 
//! This provides a simple, reliable volume control interface that works with
//! any audio object (sinks, devices, filters) via the wpctl command.

use axum::{
    extract::{Path, State},
    Json,
};
use std::sync::Arc;

use crate::api_server::{ApiError, AppState};
use super::types::*;

/// List all objects with volume control
pub async fn list_all_volumes(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<Vec<VolumeInfo>>, ApiError> {
    let volumes = crate::wpctl::list_volumes()
        .map_err(|e| ApiError::Internal(format!("Failed to list volumes: {}", e)))?;
    
    // Convert wpctl::VolumeInfo to api::VolumeInfo
    let result: Vec<VolumeInfo> = volumes.into_iter().map(|v| VolumeInfo {
        id: v.id,
        name: v.name,
        object_type: v.object_type,
        volume: Some(v.volume),
    }).collect();
    
    Ok(Json(result))
}

/// Get volume for a specific ID
pub async fn get_volume_by_id(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> Result<Json<VolumeInfo>, ApiError> {
    let volume = crate::wpctl::get_volume(id)
        .map_err(|e| {
            if e.contains("not found") {
                ApiError::NotFound(format!("Object {} not found", id))
            } else {
                ApiError::Internal(format!("Failed to get volume: {}", e))
            }
        })?;
    
    Ok(Json(VolumeInfo {
        id: volume.id,
        name: volume.name,
        object_type: volume.object_type,
        volume: Some(volume.volume),
    }))
}

/// Set volume for a specific ID
pub async fn set_volume_by_id(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<u32>,
    Json(request): Json<SetVolumeRequest>,
) -> Result<Json<VolumeResponse>, ApiError> {
    let volume = crate::wpctl::set_volume(id, request.volume)
        .map_err(|e| {
            if e.contains("not found") {
                ApiError::NotFound(format!("Object {} not found", id))
            } else {
                ApiError::Internal(format!("Failed to set volume: {}", e))
            }
        })?;
    
    Ok(Json(VolumeResponse { volume: Some(volume) }))
}

/// Save all current volumes to state file
pub async fn save_all_volumes(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Get all current volumes
    let volumes = crate::wpctl::list_volumes()
        .map_err(|e| ApiError::Internal(format!("Failed to list volumes: {}", e)))?;
    
    // Convert to state format
    let states: Vec<crate::config::VolumeState> = volumes
        .into_iter()
        .map(|v| crate::config::VolumeState {
            name: v.name,
            volume: v.volume,
        })
        .collect();
    
    // Save to state file
    crate::config::save_volume_state(states)
        .map_err(|e| ApiError::Internal(format!("Failed to save volume state: {}", e)))?;
    
    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Volume state saved"
    })))
}

/// Save a specific volume to state file
pub async fn save_volume(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Get current volume for this ID
    let volume = crate::wpctl::get_volume(id)
        .map_err(|e| {
            if e.contains("not found") {
                ApiError::NotFound(format!("Object {} not found", id))
            } else {
                ApiError::Internal(format!("Failed to get volume: {}", e))
            }
        })?;
    
    // Save to state file using name
    crate::config::save_single_volume_state(volume.name.clone(), volume.volume)
        .map_err(|e| ApiError::Internal(format!("Failed to save volume state: {}", e)))?;
    
    Ok(Json(serde_json::json!({
        "success": true,
        "id": id,
        "name": volume.name,
        "volume": volume.volume,
        "message": "Volume state saved"
    })))
}

/// Get information about the default audio sink
pub async fn get_default_sink(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<DefaultNodeInfo>, ApiError> {
    let info = crate::wpctl::get_default_sink()
        .map_err(|e| ApiError::Internal(format!("Failed to get default sink: {}", e)))?;
    
    Ok(Json(DefaultNodeInfo {
        id: info.id,
        name: info.name,
        description: info.description,
        media_class: info.media_class,
    }))
}

/// Get information about the default audio source
pub async fn get_default_source(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<DefaultNodeInfo>, ApiError> {
    let info = crate::wpctl::get_default_source()
        .map_err(|e| ApiError::Internal(format!("Failed to get default source: {}", e)))?;
    
    Ok(Json(DefaultNodeInfo {
        id: info.id,
        name: info.name,
        description: info.description,
        media_class: info.media_class,
    }))
}

/// Response for default node information
#[derive(Debug, serde::Serialize)]
pub struct DefaultNodeInfo {
    pub id: u32,
    pub name: String,
    pub description: Option<String>,
    pub media_class: Option<String>,
}
