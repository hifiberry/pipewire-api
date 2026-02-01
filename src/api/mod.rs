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
