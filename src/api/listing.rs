//! Listing handlers for PipeWire objects

use axum::{
    extract::State,
    Json,
};
use std::sync::Arc;
use std::cell::RefCell;
use std::rc::Rc;

use crate::api_server::{ApiError, AppState};
use crate::PipeWireClient;
use super::types::*;

/// List all PipeWire objects
pub async fn list_all(State(_state): State<Arc<AppState>>) -> Result<Json<ListResponse>, ApiError> {
    use pipewire as pw;
    
    let client = PipeWireClient::new()
        .map_err(|e| ApiError::Internal(format!("Failed to connect to PipeWire: {}", e)))?;
    
    let found_objects: Rc<RefCell<Vec<PipeWireObject>>> = Rc::new(RefCell::new(Vec::new()));
    let found_objects_clone = found_objects.clone();
    
    // Set up timeout
    let timeout_mainloop = client.mainloop().clone();
    let _timer = client.mainloop().loop_().add_timer(move |_| {
        timeout_mainloop.quit();
    });
    _timer.update_timer(Some(std::time::Duration::from_secs(2)), None);
    
    let _listener = client.registry()
        .add_listener_local()
        .global({
            move |global| {
                if let Some(props) = &global.props {
                    let obj_type = match global.type_ {
                        pw::types::ObjectType::Node => TYPE_NODE,
                        pw::types::ObjectType::Device => TYPE_DEVICE,
                        pw::types::ObjectType::Port => TYPE_PORT,
                        pw::types::ObjectType::Link => TYPE_LINK,
                        pw::types::ObjectType::Client => TYPE_CLIENT,
                        pw::types::ObjectType::Factory => TYPE_FACTORY,
                        pw::types::ObjectType::Module => TYPE_MODULE,
                        _ => "other",
                    };
                    
                    let name = props.get("node.name")
                        .or_else(|| props.get("device.name"))
                        .or_else(|| props.get("port.name"))
                        .or_else(|| props.get("client.name"))
                        .or_else(|| props.get("factory.name"))
                        .or_else(|| props.get("module.name"))
                        .or_else(|| props.get("object.path"))
                        .unwrap_or("unknown");
                    
                    found_objects_clone.borrow_mut().push(PipeWireObject {
                        id: global.id,
                        name: name.to_string(),
                        object_type: obj_type.to_string(),
                    });
                }
            }
        })
        .register();
    
    client.mainloop().run();
    
    let objects = found_objects.borrow().clone();
    Ok(Json(ListResponse { objects }))
}

/// Generic function to list objects by type
async fn list_by_type(state: Arc<AppState>, object_type: &str) -> Result<Json<ListResponse>, ApiError> {
    let all = list_all(State(state)).await?;
    let filtered: Vec<PipeWireObject> = all.0.objects.into_iter()
        .filter(|obj| obj.object_type == object_type)
        .collect();
    Ok(Json(ListResponse { objects: filtered }))
}

/// List all nodes
pub async fn list_nodes(State(state): State<Arc<AppState>>) -> Result<Json<ListResponse>, ApiError> {
    list_by_type(state, TYPE_NODE).await
}

/// List all devices
pub async fn list_devices(State(state): State<Arc<AppState>>) -> Result<Json<ListResponse>, ApiError> {
    list_by_type(state, TYPE_DEVICE).await
}

/// List all ports
pub async fn list_ports(State(state): State<Arc<AppState>>) -> Result<Json<ListResponse>, ApiError> {
    list_by_type(state, TYPE_PORT).await
}

/// List all modules
pub async fn list_modules(State(state): State<Arc<AppState>>) -> Result<Json<ListResponse>, ApiError> {
    list_by_type(state, TYPE_MODULE).await
}

/// List all factories
pub async fn list_factories(State(state): State<Arc<AppState>>) -> Result<Json<ListResponse>, ApiError> {
    list_by_type(state, TYPE_FACTORY).await
}

/// List all clients
pub async fn list_clients(State(state): State<Arc<AppState>>) -> Result<Json<ListResponse>, ApiError> {
    list_by_type(state, TYPE_CLIENT).await
}

/// List all links
pub async fn list_links(State(state): State<Arc<AppState>>) -> Result<Json<ListResponse>, ApiError> {
    list_by_type(state, TYPE_LINK).await
}
