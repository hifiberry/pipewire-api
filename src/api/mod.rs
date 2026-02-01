//! API module for pipewire-api
//! 
//! This module is organized into functional submodules:
//! - `types`: Common data structures
//! - `listing`: List PipeWire objects
//! - `properties`: Object properties
//! - `volume`: Unified volume control (via wpctl)
//! - `links`: Link management (via pw-link)

pub mod types;
pub mod listing;
pub mod properties;
pub mod volume;
pub mod links;

use axum::{
    routing::{get, post, put, delete},
    Json, Router,
};
use serde::Serialize;
use std::sync::Arc;
use crate::api_server::AppState;

// Re-export types for convenience
pub use types::*;

/// Response for /api/v1 endpoint listing all available endpoints
#[derive(Debug, Serialize)]
pub struct EndpointListResponse {
    pub version: &'static str,
    pub endpoints: Vec<EndpointInfo>,
}

#[derive(Debug, Serialize)]
pub struct EndpointInfo {
    pub path: &'static str,
    pub methods: Vec<&'static str>,
    pub description: &'static str,
}

/// Handler for GET /api/v1 - lists all available endpoints
pub async fn list_endpoints() -> Json<EndpointListResponse> {
    Json(EndpointListResponse {
        version: "1.0",
        endpoints: vec![
            // Core endpoints
            EndpointInfo {
                path: "/api/v1",
                methods: vec!["GET"],
                description: "List all available API endpoints",
            },
            EndpointInfo {
                path: "/api/v1/ls",
                methods: vec!["GET"],
                description: "List all PipeWire objects",
            },
            EndpointInfo {
                path: "/api/v1/objects/:id",
                methods: vec!["GET"],
                description: "Get object by ID",
            },
            EndpointInfo {
                path: "/api/v1/cache/refresh",
                methods: vec!["POST"],
                description: "Refresh object cache",
            },
            EndpointInfo {
                path: "/api/v1/properties",
                methods: vec!["GET"],
                description: "List all objects with properties",
            },
            EndpointInfo {
                path: "/api/v1/properties/:id",
                methods: vec!["GET"],
                description: "Get properties for object by ID",
            },
            // Volume endpoints
            EndpointInfo {
                path: "/api/v1/volume",
                methods: vec!["GET"],
                description: "List all volumes",
            },
            EndpointInfo {
                path: "/api/v1/volume/:id",
                methods: vec!["GET", "PUT"],
                description: "Get/set volume by ID",
            },
            EndpointInfo {
                path: "/api/v1/volume/save",
                methods: vec!["POST"],
                description: "Save all volumes to state file",
            },
            EndpointInfo {
                path: "/api/v1/volume/save/:id",
                methods: vec!["POST"],
                description: "Save specific volume to state file",
            },
            // Link endpoints
            EndpointInfo {
                path: "/api/v1/links",
                methods: vec!["GET", "POST"],
                description: "List links / Create link",
            },
            EndpointInfo {
                path: "/api/v1/links/:id",
                methods: vec!["DELETE"],
                description: "Remove link by ID",
            },
            EndpointInfo {
                path: "/api/v1/links/by-name",
                methods: vec!["DELETE"],
                description: "Remove link by port names",
            },
            EndpointInfo {
                path: "/api/v1/links/exists",
                methods: vec!["GET"],
                description: "Check if link exists",
            },
            EndpointInfo {
                path: "/api/v1/links/ports/output",
                methods: vec!["GET"],
                description: "List output ports",
            },
            EndpointInfo {
                path: "/api/v1/links/ports/input",
                methods: vec!["GET"],
                description: "List input ports",
            },
            // SpeakerEQ module endpoints
            EndpointInfo {
                path: "/api/module/speakereq/structure",
                methods: vec!["GET"],
                description: "Get SpeakerEQ DSP structure",
            },
            EndpointInfo {
                path: "/api/module/speakereq/config",
                methods: vec!["GET"],
                description: "Get SpeakerEQ configuration",
            },
            EndpointInfo {
                path: "/api/module/speakereq/io",
                methods: vec!["GET"],
                description: "Get SpeakerEQ I/O configuration",
            },
            EndpointInfo {
                path: "/api/module/speakereq/status",
                methods: vec!["GET"],
                description: "Get SpeakerEQ complete status",
            },
            EndpointInfo {
                path: "/api/module/speakereq/eq/:block/:band",
                methods: vec!["GET", "PUT"],
                description: "Get/set EQ band parameters",
            },
            EndpointInfo {
                path: "/api/module/speakereq/eq/:block/:band/enabled",
                methods: vec!["PUT"],
                description: "Enable/disable EQ band",
            },
            EndpointInfo {
                path: "/api/module/speakereq/eq/:block/clear",
                methods: vec!["PUT"],
                description: "Clear all EQ bands in block",
            },
            EndpointInfo {
                path: "/api/module/speakereq/gain/master",
                methods: vec!["GET", "PUT"],
                description: "Get/set master gain",
            },
            EndpointInfo {
                path: "/api/module/speakereq/enable",
                methods: vec!["GET", "PUT"],
                description: "Get/set enable status",
            },
            EndpointInfo {
                path: "/api/module/speakereq/refresh",
                methods: vec!["POST"],
                description: "Refresh parameter cache",
            },
            EndpointInfo {
                path: "/api/module/speakereq/default",
                methods: vec!["POST"],
                description: "Reset to default settings",
            },
            // RIAA module endpoints
            EndpointInfo {
                path: "/api/module/riaa/config",
                methods: vec!["GET"],
                description: "Get all RIAA settings",
            },
            EndpointInfo {
                path: "/api/module/riaa/gain",
                methods: vec!["GET", "PUT"],
                description: "Get/set RIAA gain",
            },
            EndpointInfo {
                path: "/api/module/riaa/subsonic",
                methods: vec!["GET", "PUT"],
                description: "Get/set subsonic filter",
            },
            EndpointInfo {
                path: "/api/module/riaa/riaa-enable",
                methods: vec!["GET", "PUT"],
                description: "Enable/disable RIAA equalization",
            },
            EndpointInfo {
                path: "/api/module/riaa/declick",
                methods: vec!["GET", "PUT"],
                description: "Enable/disable declicker",
            },
            EndpointInfo {
                path: "/api/module/riaa/spike",
                methods: vec!["GET", "PUT"],
                description: "Get/set spike detection config",
            },
            EndpointInfo {
                path: "/api/module/riaa/notch",
                methods: vec!["GET", "PUT"],
                description: "Get/set notch filter config",
            },
            EndpointInfo {
                path: "/api/module/riaa/set-default",
                methods: vec!["PUT"],
                description: "Reset RIAA to defaults",
            },
            // Graph endpoints
            EndpointInfo {
                path: "/api/v1/graph",
                methods: vec!["GET"],
                description: "Get audio topology graph (DOT format)",
            },
            EndpointInfo {
                path: "/api/v1/graph/png",
                methods: vec!["GET"],
                description: "Get audio topology graph (PNG image)",
            },
        ],
    })
}

/// Create router for all API endpoints
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        // API root - list all endpoints
        .route("/api/v1", get(list_endpoints))
        // Listing endpoints
        .route("/api/v1/ls", get(listing::list_all))
        // Object by ID endpoint
        .route("/api/v1/objects/:id", get(listing::get_object_by_id))
        // Cache refresh endpoint
        .route("/api/v1/cache/refresh", post(listing::refresh_cache))
        // Properties endpoints
        .route("/api/v1/properties", get(properties::list_all_properties))
        .route("/api/v1/properties/:id", get(properties::get_object_properties))
        // Unified volume endpoints (via wpctl)
        .route("/api/v1/volume", get(volume::list_all_volumes))
        .route("/api/v1/volume/:id", get(volume::get_volume_by_id))
        .route("/api/v1/volume/:id", put(volume::set_volume_by_id))
        .route("/api/v1/volume/save", post(volume::save_all_volumes))
        .route("/api/v1/volume/save/:id", post(volume::save_volume))
        // Links endpoints (via pw-link)
        .route("/api/v1/links", get(links::list_links))
        .route("/api/v1/links", post(links::create_link))
        .route("/api/v1/links/:id", delete(links::remove_link_by_id))
        .route("/api/v1/links/by-name", delete(links::remove_link_by_name))
        .route("/api/v1/links/exists", get(links::check_link_exists))
        .route("/api/v1/links/ports/output", get(links::list_output_ports))
        .route("/api/v1/links/ports/input", get(links::list_input_ports))
        .with_state(state)
}
