use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::collections::HashMap;
use crate::api_server::{ApiError, AppState};
use serde_json::Value as JsonValue;

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic_properties: Option<HashMap<String, JsonValue>>,
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
                        dynamic_properties: None,
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
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;
    use libspa::param::ParamType;
    
    let client = PipeWireClient::new()
        .map_err(|e| ApiError::Internal(format!("Failed to connect to PipeWire: {}", e)))?;
    
    let found_object: Rc<RefCell<Option<PipeWireObjectWithProperties>>> = Rc::new(RefCell::new(None));
    let found_object_clone = found_object.clone();
    
    // Store node reference for parameter reading
    let node_ref: Rc<RefCell<Option<pw::node::Node>>> = Rc::new(RefCell::new(None));
    let node_ref_clone = node_ref.clone();
    let node_ref_for_params = node_ref.clone();
    
    let done = Rc::new(Cell::new(false));
    let done_clone = done.clone();
    let mainloop_clone = client.mainloop().clone();
    
    // Set up timeout
    let timeout_mainloop = client.mainloop().clone();
    let timeout_done = done.clone();
    let _timer = client.mainloop().loop_().add_timer(move |_| {
        if !timeout_done.get() {
            timeout_mainloop.quit();
        }
    });
    _timer.update_timer(Some(std::time::Duration::from_millis(500)), None);
    
    let _registry_listener = client.registry()
        .add_listener_local()
        .global({
            let registry_weak = client.registry().downgrade();
            move |global| {
                if global.id == id {
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
                        
                        *found_object_clone.borrow_mut() = Some(PipeWireObjectWithProperties {
                            id: global.id,
                            name: name.to_string(),
                            object_type: obj_type.to_string(),
                            properties,
                            dynamic_properties: None,
                        });
                        
                        // If it's a node, bind it to read parameters
                        if matches!(global.type_, pw::types::ObjectType::Node) {
                            if let Some(reg) = registry_weak.upgrade() {
                                if let Ok(node) = reg.bind::<pw::node::Node, _>(&global) {
                                    *node_ref_clone.borrow_mut() = Some(node);
                                }
                            }
                        }
                        
                        done_clone.set(true);
                        mainloop_clone.quit();
                    }
                }
            }
        })
        .register();
    
    client.mainloop().run();
    
    if !done.get() {
        return Err(ApiError::NotFound(format!("Object with id {} not found", id)));
    }
    
    // If we have a node, fetch dynamic properties
    let dynamic_props: Option<HashMap<String, JsonValue>> = if let Some(ref node) = *node_ref_for_params.borrow() {
        let params_map: Rc<RefCell<HashMap<String, JsonValue>>> = Rc::new(RefCell::new(HashMap::new()));
        let params_map_clone = params_map.clone();
        
        let param_done = Rc::new(Cell::new(false));
        let param_done_for_timer = param_done.clone();
        let param_done_for_listener = param_done.clone();
        
        let timeout_mainloop2 = client.mainloop().clone();
        let _timer2 = client.mainloop().loop_().add_timer(move |_| {
            if !param_done_for_timer.get() {
                timeout_mainloop2.quit();
            }
        });
        _timer2.update_timer(Some(std::time::Duration::from_millis(300)), None);
        
        let mainloop_for_param = client.mainloop().clone();
        let _param_listener = node
            .add_listener_local()
            .param(move |_, param_type, _, _, param_pod| {
                if param_type != ParamType::Props {
                    return;
                }
                
                if let Some(pod) = param_pod {
                    let parsed = crate::pod_parser::parse_props_pod(pod);
                    params_map_clone.borrow_mut().extend(parsed);
                }
                
                param_done_for_listener.set(true);
                mainloop_for_param.quit();
            })
            .register();
        
        node.enum_params(0, Some(ParamType::Props), 0, u32::MAX);
        client.mainloop().run();
        
        let params = params_map.borrow().clone();
        if params.is_empty() {
            None
        } else {
            Some(params)
        }
    } else {
        None
    };
    
    // Combine results
    let obj_opt = found_object.borrow().clone();
    if let Some(mut obj) = obj_opt {
        obj.dynamic_properties = dynamic_props;
        Ok(Json(obj))
    } else {
        Err(ApiError::NotFound(format!("Object with id {} not found", id)))
    }
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
