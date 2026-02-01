//! Links API handlers - create, list, and remove PipeWire links
//!
//! This module provides REST API endpoints for managing PipeWire audio links.

use axum::{
    extract::{Path, State},
    Json,
};
use std::sync::Arc;
use serde::{Deserialize, Serialize};

use crate::api_server::{ApiError, AppState};
use crate::pwlink;

/// Request to create a link
#[derive(Debug, Clone, Deserialize)]
pub struct CreateLinkRequest {
    /// Output port name (format: "node_name:port_name") or port ID
    pub output: String,
    /// Input port name (format: "node_name:port_name") or port ID
    pub input: String,
}

/// Response for link operations
#[derive(Debug, Clone, Serialize)]
pub struct LinkResponse {
    pub status: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_id: Option<u32>,
}

/// A link in the list response
#[derive(Debug, Clone, Serialize)]
pub struct LinkInfo {
    pub id: u32,
    pub output_port_id: u32,
    pub output_port_name: String,
    pub input_port_id: u32,
    pub input_port_name: String,
}

/// Response for list links
#[derive(Debug, Clone, Serialize)]
pub struct ListLinksResponse {
    pub links: Vec<LinkInfo>,
}

/// A port in the list response
#[derive(Debug, Clone, Serialize)]
pub struct PortInfo {
    pub id: u32,
    pub name: String,
    pub node_name: String,
    pub port_name: String,
}

/// Response for list ports
#[derive(Debug, Clone, Serialize)]
pub struct ListPortsResponse {
    pub ports: Vec<PortInfo>,
}

/// List all links
/// GET /api/v1/links
pub async fn list_links(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<ListLinksResponse>, ApiError> {
    let links = pwlink::list_links()
        .map_err(|e| ApiError::Internal(format!("Failed to list links: {}", e)))?;
    
    let link_infos: Vec<LinkInfo> = links.iter()
        .map(|l| LinkInfo {
            id: l.id,
            output_port_id: l.output_port_id,
            output_port_name: l.output_port_name.clone(),
            input_port_id: l.input_port_id,
            input_port_name: l.input_port_name.clone(),
        })
        .collect();
    
    Ok(Json(ListLinksResponse { links: link_infos }))
}

/// List output ports
/// GET /api/v1/links/ports/output
pub async fn list_output_ports(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<ListPortsResponse>, ApiError> {
    let ports = pwlink::list_output_ports()
        .map_err(|e| ApiError::Internal(format!("Failed to list output ports: {}", e)))?;
    
    let port_infos: Vec<PortInfo> = ports.iter()
        .map(|p| PortInfo {
            id: p.id,
            name: p.name.clone(),
            node_name: p.node_name.clone(),
            port_name: p.port_name.clone(),
        })
        .collect();
    
    Ok(Json(ListPortsResponse { ports: port_infos }))
}

/// List input ports
/// GET /api/v1/links/ports/input
pub async fn list_input_ports(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<ListPortsResponse>, ApiError> {
    let ports = pwlink::list_input_ports()
        .map_err(|e| ApiError::Internal(format!("Failed to list input ports: {}", e)))?;
    
    let port_infos: Vec<PortInfo> = ports.iter()
        .map(|p| PortInfo {
            id: p.id,
            name: p.name.clone(),
            node_name: p.node_name.clone(),
            port_name: p.port_name.clone(),
        })
        .collect();
    
    Ok(Json(ListPortsResponse { ports: port_infos }))
}

/// Create a link between two ports
/// POST /api/v1/links
pub async fn create_link(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<CreateLinkRequest>,
) -> Result<Json<LinkResponse>, ApiError> {
    // Check if it looks like a port ID (all digits)
    let is_output_id = request.output.chars().all(|c| c.is_ascii_digit());
    let is_input_id = request.input.chars().all(|c| c.is_ascii_digit());
    
    let result = if is_output_id && is_input_id {
        // Both are IDs
        let output_id: u32 = request.output.parse()
            .map_err(|_| ApiError::BadRequest("Invalid output port ID".to_string()))?;
        let input_id: u32 = request.input.parse()
            .map_err(|_| ApiError::BadRequest("Invalid input port ID".to_string()))?;
        pwlink::create_link_by_id(output_id, input_id)
    } else {
        // Use names
        pwlink::create_link(&request.output, &request.input)
    };
    
    match result {
        Ok(()) => {
            // Try to find the link ID
            let link_id = pwlink::find_link(&request.output, &request.input)
                .ok()
                .flatten()
                .map(|l| l.id);
            
            Ok(Json(LinkResponse {
                status: "ok".to_string(),
                message: format!("Link created: {} -> {}", request.output, request.input),
                link_id,
            }))
        }
        Err(e) => {
            Err(ApiError::Internal(format!("Failed to create link: {}", e)))
        }
    }
}

/// Remove a link by ID
/// DELETE /api/v1/links/:id
pub async fn remove_link_by_id(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> Result<Json<LinkResponse>, ApiError> {
    pwlink::remove_link(id)
        .map_err(|e| ApiError::Internal(format!("Failed to remove link: {}", e)))?;
    
    Ok(Json(LinkResponse {
        status: "ok".to_string(),
        message: format!("Link {} removed", id),
        link_id: Some(id),
    }))
}

/// Remove a link between two ports by name
/// DELETE /api/v1/links/by-name
#[derive(Debug, Clone, Deserialize)]
pub struct RemoveLinkByNameRequest {
    pub output: String,
    pub input: String,
}

pub async fn remove_link_by_name(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<RemoveLinkByNameRequest>,
) -> Result<Json<LinkResponse>, ApiError> {
    pwlink::remove_link_by_name(&request.output, &request.input)
        .map_err(|e| ApiError::Internal(format!("Failed to remove link: {}", e)))?;
    
    Ok(Json(LinkResponse {
        status: "ok".to_string(),
        message: format!("Link removed: {} -> {}", request.output, request.input),
        link_id: None,
    }))
}

/// Check if a link exists between two ports
/// GET /api/v1/links/exists?output=...&input=...
#[derive(Debug, Clone, Deserialize)]
pub struct LinkExistsQuery {
    pub output: String,
    pub input: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LinkExistsResponse {
    pub exists: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_id: Option<u32>,
}

pub async fn check_link_exists(
    State(_state): State<Arc<AppState>>,
    axum::extract::Query(query): axum::extract::Query<LinkExistsQuery>,
) -> Result<Json<LinkExistsResponse>, ApiError> {
    let link = pwlink::find_link(&query.output, &query.input)
        .map_err(|e| ApiError::Internal(format!("Failed to check link: {}", e)))?;
    
    Ok(Json(LinkExistsResponse {
        exists: link.is_some(),
        link_id: link.map(|l| l.id),
    }))
}
