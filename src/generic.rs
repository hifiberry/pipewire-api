use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::collections::HashMap;
use crate::api_server::{ApiError, AppState};

// Object type constants
const TYPE_NODE: &str = "node";
const TYPE_DEVICE: &str = "device";
const TYPE_PORT: &str = "port";
const TYPE_MODULE: &str = "module";
const TYPE_FACTORY: &str = "factory";
const TYPE_CLIENT: &str = "client";
const TYPE_LINK: &str = "link";

// Response types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipeWireObject {
    pub id: u32,
    pub name: String,
    #[serde(rename = "type")]
    pub object_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListResponse {
    pub objects: Vec<PipeWireObject>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipeWireObjectWithProperties {
    pub id: u32,
    pub name: String,
    #[serde(rename = "type")]
    pub object_type: String,
    pub properties: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PropertiesResponse {
    pub objects: Vec<PipeWireObjectWithProperties>,
}

// Handlers
pub async fn list_all(State(_state): State<Arc<AppState>>) -> Result<Json<ListResponse>, ApiError> {
    use crate::PipeWireClient;
    use pipewire as pw;
    use std::cell::RefCell;
    use std::rc::Rc;
    
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

// Generic function to list objects by type
async fn list_by_type(state: Arc<AppState>, object_type: &str) -> Result<Json<ListResponse>, ApiError> {
    let all = list_all(State(state)).await?;
    let filtered: Vec<PipeWireObject> = all.0.objects.into_iter()
        .filter(|obj| obj.object_type == object_type)
        .collect();
    Ok(Json(ListResponse { objects: filtered }))
}

pub async fn list_nodes(State(state): State<Arc<AppState>>) -> Result<Json<ListResponse>, ApiError> {
    list_by_type(state, TYPE_NODE).await
}

pub async fn list_devices(State(state): State<Arc<AppState>>) -> Result<Json<ListResponse>, ApiError> {
    list_by_type(state, TYPE_DEVICE).await
}

pub async fn list_ports(State(state): State<Arc<AppState>>) -> Result<Json<ListResponse>, ApiError> {
    list_by_type(state, TYPE_PORT).await
}

pub async fn list_modules(State(state): State<Arc<AppState>>) -> Result<Json<ListResponse>, ApiError> {
    list_by_type(state, TYPE_MODULE).await
}

pub async fn list_factories(State(state): State<Arc<AppState>>) -> Result<Json<ListResponse>, ApiError> {
    list_by_type(state, TYPE_FACTORY).await
}

pub async fn list_clients(State(state): State<Arc<AppState>>) -> Result<Json<ListResponse>, ApiError> {
    list_by_type(state, TYPE_CLIENT).await
}

pub async fn list_links(State(state): State<Arc<AppState>>) -> Result<Json<ListResponse>, ApiError> {
    list_by_type(state, TYPE_LINK).await
}

pub async fn list_all_properties(State(_state): State<Arc<AppState>>) -> Result<Json<PropertiesResponse>, ApiError> {
    use crate::PipeWireClient;
    use pipewire as pw;
    use std::cell::RefCell;
    use std::rc::Rc;
    
    let client = PipeWireClient::new()
        .map_err(|e| ApiError::Internal(format!("Failed to connect to PipeWire: {}", e)))?;
    
    let found_objects: Rc<RefCell<Vec<PipeWireObjectWithProperties>>> = Rc::new(RefCell::new(Vec::new()));
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
                    
                    // Collect all properties
                    let mut properties = HashMap::new();
                    for (key, value) in props.iter() {
                        properties.insert(key.to_string(), value.to_string());
                    }
                    
                    found_objects_clone.borrow_mut().push(PipeWireObjectWithProperties {
                        id: global.id,
                        name: name.to_string(),
                        object_type: obj_type.to_string(),
                        properties,
                    });
                }
            }
        })
        .register();
    
    client.mainloop().run();
    
    let objects = found_objects.borrow().clone();
    Ok(Json(PropertiesResponse { objects }))
}

pub async fn get_object_properties(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> Result<Json<PipeWireObjectWithProperties>, ApiError> {
    use crate::PipeWireClient;
    use pipewire as pw;
    use std::cell::RefCell;
    use std::rc::Rc;
    use libspa::param::ParamType;
    
    let client = PipeWireClient::new()
        .map_err(|e| ApiError::Internal(format!("Failed to connect to PipeWire: {}", e)))?;
    
    let found_object: Rc<RefCell<Option<PipeWireObjectWithProperties>>> = Rc::new(RefCell::new(None));
    let found_object_clone = found_object.clone();
    let found_object_for_params = found_object.clone();
    
    // Store node type to determine if we should read parameters
    let is_node: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));
    let is_node_clone = is_node.clone();
    
    // Set up timeout
    let timeout_mainloop = client.mainloop().clone();
    let _timer = client.mainloop().loop_().add_timer(move |_| {
        timeout_mainloop.quit();
    });
    _timer.update_timer(Some(std::time::Duration::from_secs(2)), None);
    
    let _registry_listener = client.registry()
        .add_listener_local()
        .global({
            move |global| {
                if global.id == id {
                    if let Some(props) = &global.props {
                        let obj_type = match global.type_ {
                            pw::types::ObjectType::Node => {
                                *is_node_clone.borrow_mut() = true;
                                TYPE_NODE
                            }
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
                        
                        // Collect all properties
                        let mut properties = HashMap::new();
                        for (key, value) in props.iter() {
                            properties.insert(key.to_string(), value.to_string());
                        }
                        
                        *found_object_clone.borrow_mut() = Some(PipeWireObjectWithProperties {
                            id: global.id,
                            name: name.to_string(),
                            object_type: obj_type.to_string(),
                            properties,
                        });
                    }
                }
            }
        })
        .register();
    
    client.mainloop().run();
    
    let result = found_object.borrow().clone();
    result
        .ok_or_else(|| ApiError::NotFound(format!("Object with id {} not found", id)))
        .map(Json)
}

// Create router for generic API endpoints
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/v1/ls", get(list_all))
        .route("/api/v1/ls/nodes", get(list_nodes))
        .route("/api/v1/ls/devices", get(list_devices))
        .route("/api/v1/ls/ports", get(list_ports))
        .route("/api/v1/ls/modules", get(list_modules))
        .route("/api/v1/ls/factories", get(list_factories))
        .route("/api/v1/ls/clients", get(list_clients))
        .route("/api/v1/ls/links", get(list_links))
        .route("/api/v1/properties", get(list_all_properties))
        .route("/api/v1/properties/:id", get(get_object_properties))
        .with_state(state)
}
