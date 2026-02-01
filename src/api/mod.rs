//! API module for pipewire-api
//! 
//! This module is organized into functional submodules:
//! - `types`: Common data structures
//! - `listing`: List PipeWire objects
//! - `properties`: Object properties
//! - `volume`: Unified volume control (via wpctl)

pub mod types;
pub mod listing;
pub mod properties;
pub mod volume;

use axum::{
    routing::{get, post, put},
    Router,
};
use std::sync::Arc;
use crate::api_server::AppState;

// Re-export types for convenience
pub use types::*;

/// Create router for all API endpoints
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        // Listing endpoints
        .route("/api/v1/ls", get(listing::list_all))
        .route("/api/v1/ls/nodes", get(listing::list_nodes))
        .route("/api/v1/ls/devices", get(listing::list_devices))
        .route("/api/v1/ls/ports", get(listing::list_ports))
        .route("/api/v1/ls/modules", get(listing::list_modules))
        .route("/api/v1/ls/factories", get(listing::list_factories))
        .route("/api/v1/ls/clients", get(listing::list_clients))
        .route("/api/v1/ls/links", get(listing::list_links))
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
        .with_state(state)
}
