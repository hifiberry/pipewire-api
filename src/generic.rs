use axum::{
    extract::{Path, State},
    routing::{get, post, put},
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: u32,
    pub name: String,
    pub properties: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SinkInfo {
    pub id: u32,
    pub name: String,
    pub properties: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeInfo {
    pub id: u32,
    pub name: String,
    pub object_type: String, // "device" or "sink"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SetVolumeRequest {
    pub volume: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VolumeResponse {
    pub volume: Option<f32>,
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

// List all devices with volume information
pub async fn list_devices_with_info(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<Vec<DeviceInfo>>, ApiError> {
    use crate::PipeWireClient;
    use pipewire as pw;
    use std::cell::RefCell;
    use std::rc::Rc;
    use libspa::param::ParamType;
    
    let client = PipeWireClient::new()
        .map_err(|e| ApiError::Internal(format!("Failed to connect to PipeWire: {}", e)))?;
    
    // Collect all devices with bound Device objects during the initial scan
    let devices: Rc<RefCell<Vec<(u32, HashMap<String, String>, pw::device::Device)>>> = 
        Rc::new(RefCell::new(Vec::new()));
    let devices_clone = devices.clone();
    
    let registry_for_bind = client.registry().downgrade();
    let _listener = client.registry()
        .add_listener_local()
        .global(move |global| {
            if global.type_ == pw::types::ObjectType::Device {
                if let Some(props) = &global.props {
                    if let Some(reg) = registry_for_bind.upgrade() {
                        if let Ok(dev) = reg.bind::<pw::device::Device, _>(&global) {
                            let mut properties = HashMap::new();
                            for (key, value) in props.iter() {
                                properties.insert(key.to_string(), value.to_string());
                            }
                            devices_clone.borrow_mut().push((global.id, properties, dev));
                        }
                    }
                }
            }
        })
        .global_remove(move |_| {})
        .register();
    
    // Set up timeout
    let timeout_mainloop = client.mainloop().clone();
    let _timer = client.mainloop().loop_().add_timer(move |_| {
        timeout_mainloop.quit();
    });
    _timer.update_timer(Some(std::time::Duration::from_millis(500)), None);
    
    client.mainloop().run();
    
    // Now read Route parameters for each device to get volumes
    let mut result_devices = Vec::new();
    
    for (device_id, properties, device) in devices.borrow().iter() {
        let name = properties.get("device.name")
            .or_else(|| properties.get("device.description"))
            .map(|s| s.as_str())
            .unwrap_or("unknown");
        
        let volume_ref: Rc<RefCell<Option<f32>>> = Rc::new(RefCell::new(None));
        let volume_ref_clone = volume_ref.clone();
        
        let mainloop_for_param = client.mainloop().clone();
        let _param_listener = device
            .add_listener_local()
            .param(move |_, param_type, _, _, param_pod| {
                if param_type != ParamType::Route {
                    return;
                }
                
                if let Some(pod) = param_pod {
                    let parsed = crate::pod_parser::parse_props_pod(pod);
                    
                    // Look for channelVolumes in the parsed data (might be nested in prop_10)
                    if let Some(JsonValue::Array(volumes)) = parsed.get("channelVolumes") {
                        if let Some(JsonValue::Number(vol)) = volumes.first() {
                            if let Some(v) = vol.as_f64() {
                                *volume_ref_clone.borrow_mut() = Some(v as f32);
                            }
                        }
                    } else if let Some(JsonValue::Object(props_obj)) = parsed.get("prop_10") {
                        // prop_10 is the props object inside the route
                        if let Some(JsonValue::Array(volumes)) = props_obj.get("channelVolumes") {
                            if let Some(JsonValue::Number(vol)) = volumes.first() {
                                if let Some(v) = vol.as_f64() {
                                    *volume_ref_clone.borrow_mut() = Some(v as f32);
                                }
                            }
                        }
                    }
                }
                
                mainloop_for_param.quit();
            })
            .register();
        
        // Query params with a timeout
        let timeout_mainloop2 = client.mainloop().clone();
        let _timer2 = client.mainloop().loop_().add_timer(move |_| {
            timeout_mainloop2.quit();
        });
        _timer2.update_timer(Some(std::time::Duration::from_millis(200)), None);
        
        device.enum_params(0, Some(ParamType::Route), 0, u32::MAX);
        client.mainloop().run();
        
        let device_info = DeviceInfo {
            id: *device_id,
            name: name.to_string(),
            properties: properties.clone(),
            volume: *volume_ref.borrow(),
        };
        
        result_devices.push(device_info);
    }
    
    Ok(Json(result_devices))
}

// Get device info including volume from Route parameters
pub async fn get_device_info(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> Result<Json<DeviceInfo>, ApiError> {
    use crate::PipeWireClient;
    use pipewire as pw;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;
    use libspa::param::ParamType;
    
    let client = PipeWireClient::new()
        .map_err(|e| ApiError::Internal(format!("Failed to connect to PipeWire: {}", e)))?;
    
    let device_ref: Rc<RefCell<Option<pw::device::Device>>> = Rc::new(RefCell::new(None));
    let device_ref_clone = device_ref.clone();
    let device_ref_for_params = device_ref.clone();
    
    let device_info: Rc<RefCell<Option<DeviceInfo>>> = Rc::new(RefCell::new(None));
    let device_info_clone = device_info.clone();
    
    let done = Rc::new(Cell::new(false));
    let done_clone = done.clone();
    let mainloop_clone = client.mainloop().clone();
    
    let registry_for_bind = client.registry().downgrade();
    let _listener = client.registry()
        .add_listener_local()
        .global(move |global| {
            if global.id == id && global.type_ == pw::types::ObjectType::Device {
                if let Some(reg) = registry_for_bind.upgrade() {
                    if let Ok(dev) = reg.bind::<pw::device::Device, _>(&global) {
                        *device_ref_clone.borrow_mut() = Some(dev);
                        
                        // Store basic device info
                        if let Some(props) = &global.props {
                            let name = props.get("device.name")
                                .or_else(|| props.get("device.description"))
                                .unwrap_or("unknown");
                            
                            let mut properties = HashMap::new();
                            for (key, value) in props.iter() {
                                properties.insert(key.to_string(), value.to_string());
                            }
                            
                            *device_info_clone.borrow_mut() = Some(DeviceInfo {
                                id: global.id,
                                name: name.to_string(),
                                properties,
                                volume: None, // Will be filled from Route params
                            });
                        }
                        
                        done_clone.set(true);
                        mainloop_clone.quit();
                    }
                }
            }
        })
        .register();
    
    // Set up timeout
    let timeout_mainloop = client.mainloop().clone();
    let timeout_done = done.clone();
    let _timer = client.mainloop().loop_().add_timer(move |_| {
        if !timeout_done.get() {
            timeout_mainloop.quit();
        }
    });
    _timer.update_timer(Some(std::time::Duration::from_millis(500)), None);
    
    client.mainloop().run();
    
    if !done.get() {
        return Err(ApiError::NotFound(format!("Device {} not found", id)));
    }
    
    // Now read Route parameters to get volume
    let volume_ref: Rc<RefCell<Option<f32>>> = Rc::new(RefCell::new(None));
    let volume_ref_clone = volume_ref.clone();
    
    let param_done = Rc::new(Cell::new(false));
    let param_done_for_timer = param_done.clone();
    let param_done_for_listener = param_done.clone();
    
    let timeout_mainloop2 = client.mainloop().clone();
    let _timer2 = client.mainloop().loop_().add_timer(move |_| {
        if !param_done_for_timer.get() {
            timeout_mainloop2.quit();
        }
    });
    _timer2.update_timer(Some(std::time::Duration::from_millis(500)), None);
    
    let device_borrow = device_ref_for_params.borrow();
    if let Some(device) = device_borrow.as_ref() {
        let mainloop_for_param = client.mainloop().clone();
        let _param_listener = device
            .add_listener_local()
            .param(move |_, param_type, _, _, param_pod| {
                if param_type != ParamType::Route {
                    return;
                }
                
                if let Some(pod) = param_pod {
                    let parsed = crate::pod_parser::parse_props_pod(pod);
                    
                    // Look for channelVolumes in the parsed data (might be nested in prop_10)
                    if let Some(JsonValue::Array(volumes)) = parsed.get("channelVolumes") {
                        if let Some(JsonValue::Number(vol)) = volumes.first() {
                            if let Some(v) = vol.as_f64() {
                                *volume_ref_clone.borrow_mut() = Some(v as f32);
                            }
                        }
                    } else if let Some(JsonValue::Object(props_obj)) = parsed.get("prop_10") {
                        // prop_10 is the props object inside the route
                        if let Some(JsonValue::Array(volumes)) = props_obj.get("channelVolumes") {
                            if let Some(JsonValue::Number(vol)) = volumes.first() {
                                if let Some(v) = vol.as_f64() {
                                    *volume_ref_clone.borrow_mut() = Some(v as f32);
                                }
                            }
                        }
                    }
                }
                
                param_done_for_listener.set(true);
                mainloop_for_param.quit();
            })
            .register();
        
        device.enum_params(0, Some(ParamType::Route), 0, u32::MAX);
        client.mainloop().run();
    }
    
    // Combine results
    let mut info = device_info.borrow().clone()
        .ok_or_else(|| ApiError::Internal("Failed to get device info".to_string()))?;
    
    info.volume = *volume_ref.borrow();
    
    Ok(Json(info))
}

// Get device volume only
pub async fn get_device_volume(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> Result<Json<VolumeResponse>, ApiError> {
    use crate::PipeWireClient;
    use pipewire as pw;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;
    use libspa::param::ParamType;
    
    let client = PipeWireClient::new()
        .map_err(|e| ApiError::Internal(format!("Failed to connect to PipeWire: {}", e)))?;
    
    let device_ref: Rc<RefCell<Option<pw::device::Device>>> = Rc::new(RefCell::new(None));
    let device_ref_clone = device_ref.clone();
    let device_ref_for_params = device_ref.clone();
    
    let done = Rc::new(Cell::new(false));
    let done_clone = done.clone();
    let mainloop_clone = client.mainloop().clone();
    
    let registry_for_bind = client.registry().downgrade();
    let _listener = client.registry()
        .add_listener_local()
        .global(move |global| {
            if global.id == id && global.type_ == pw::types::ObjectType::Device {
                if let Some(reg) = registry_for_bind.upgrade() {
                    if let Ok(dev) = reg.bind::<pw::device::Device, _>(&global) {
                        *device_ref_clone.borrow_mut() = Some(dev);
                        done_clone.set(true);
                        mainloop_clone.quit();
                    }
                }
            }
        })
        .register();
    
    // Set up timeout
    let timeout_mainloop = client.mainloop().clone();
    let timeout_done = done.clone();
    let _timer = client.mainloop().loop_().add_timer(move |_| {
        if !timeout_done.get() {
            timeout_mainloop.quit();
        }
    });
    _timer.update_timer(Some(std::time::Duration::from_millis(500)), None);
    
    client.mainloop().run();
    
    if !done.get() {
        return Err(ApiError::NotFound(format!("Device {} not found", id)));
    }
    
    // Read Route parameters to get volume
    let volume_ref: Rc<RefCell<Option<f32>>> = Rc::new(RefCell::new(None));
    let volume_ref_clone = volume_ref.clone();
    
    let param_done = Rc::new(Cell::new(false));
    let param_done_for_timer = param_done.clone();
    let param_done_for_listener = param_done.clone();
    
    let timeout_mainloop2 = client.mainloop().clone();
    let _timer2 = client.mainloop().loop_().add_timer(move |_| {
        if !param_done_for_timer.get() {
            timeout_mainloop2.quit();
        }
    });
    _timer2.update_timer(Some(std::time::Duration::from_millis(500)), None);
    
    let device_borrow = device_ref_for_params.borrow();
    if let Some(device) = device_borrow.as_ref() {
        let mainloop_for_param = client.mainloop().clone();
        let _param_listener = device
            .add_listener_local()
            .param(move |_, param_type, _, _, param_pod| {
                if param_type != ParamType::Route {
                    return;
                }
                
                if let Some(pod) = param_pod {
                    let parsed = crate::pod_parser::parse_props_pod(pod);
                    
                    // Look for channelVolumes - now should be in nested prop_10 Object
                    if let Some(JsonValue::Object(props_obj)) = parsed.get("prop_10") {
                        if let Some(JsonValue::Array(volumes)) = props_obj.get("channelVolumes") {
                            if let Some(JsonValue::Number(vol)) = volumes.first() {
                                if let Some(v) = vol.as_f64() {
                                    *volume_ref_clone.borrow_mut() = Some(v as f32);
                                }
                            }
                        }
                    }
                }
                
                param_done_for_listener.set(true);
                mainloop_for_param.quit();
            })
            .register();
        
        device.enum_params(0, Some(ParamType::Route), 0, u32::MAX);
        client.mainloop().run();
    }
    
    let volume = *volume_ref.borrow();
    Ok(Json(VolumeResponse { volume }))
}

// Set device volume via Route parameters
pub async fn set_device_volume(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<u32>,
    Json(request): Json<SetVolumeRequest>,
) -> Result<Json<DeviceInfo>, ApiError> {
    use crate::PipeWireClient;
    use pipewire as pw;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;
    use libspa::param::ParamType;
    use libspa::pod::{serialize::PodSerializer, Object, Property, Value};
    
    let client = PipeWireClient::new()
        .map_err(|e| ApiError::Internal(format!("Failed to connect to PipeWire: {}", e)))?;
    
    let device_ref: Rc<RefCell<Option<pw::device::Device>>> = Rc::new(RefCell::new(None));
    let device_ref_clone = device_ref.clone();
    
    let done = Rc::new(Cell::new(false));
    let done_clone = done.clone();
    let mainloop_clone = client.mainloop().clone();
    
    let registry_for_bind = client.registry().downgrade();
    let _listener = client.registry()
        .add_listener_local()
        .global(move |global| {
            if global.id == id && global.type_ == pw::types::ObjectType::Device {
                if let Some(reg) = registry_for_bind.upgrade() {
                    if let Ok(dev) = reg.bind::<pw::device::Device, _>(&global) {
                        *device_ref_clone.borrow_mut() = Some(dev);
                        done_clone.set(true);
                        mainloop_clone.quit();
                    }
                }
            }
        })
        .register();
    
    // Set up timeout
    let timeout_mainloop = client.mainloop().clone();
    let timeout_done = done.clone();
    let _timer = client.mainloop().loop_().add_timer(move |_| {
        if !timeout_done.get() {
            timeout_mainloop.quit();
        }
    });
    _timer.update_timer(Some(std::time::Duration::from_millis(500)), None);
    
    client.mainloop().run();
    
    if !done.get() {
        return Err(ApiError::NotFound(format!("Device {} not found", id)));
    }
    
    // Build Route parameter with updated volume
    let volume = request.volume;
    let mut buffer = vec![0u8; 4096];
    
    let props_inner = Object {
        type_: libspa::sys::SPA_TYPE_OBJECT_Props,
        id: libspa::sys::SPA_PARAM_Route,
        properties: vec![
            Property {
                key: 65540, // mute
                flags: libspa::pod::PropertyFlags::empty(),
                value: Value::Bool(false),
            },
            Property {
                key: 65544, // channelVolumes
                flags: libspa::pod::PropertyFlags::empty(),
                value: Value::ValueArray(libspa::pod::ValueArray::Float(vec![volume, volume])),
            },
            Property {
                key: 65547, // channelMap
                flags: libspa::pod::PropertyFlags::empty(),
                value: Value::ValueArray(libspa::pod::ValueArray::Id(vec![
                    libspa::utils::Id(3), // FL
                    libspa::utils::Id(4), // FR
                ])),
            },
        ],
    };
    
    let route_object = Object {
        type_: 262153, // SPA_TYPE_OBJECT_ParamRoute
        id: libspa::sys::SPA_PARAM_Route,
        properties: vec![
            Property {
                key: 1, // index
                flags: libspa::pod::PropertyFlags::empty(),
                value: Value::Int(0), // route index 0
            },
            Property {
                key: 2, // direction
                flags: libspa::pod::PropertyFlags::empty(),
                value: Value::Id(libspa::utils::Id(1)), // Output
            },
            Property {
                key: 3, // device
                flags: libspa::pod::PropertyFlags::empty(),
                value: Value::Int(1),
            },
            Property {
                key: 10, // props
                flags: libspa::pod::PropertyFlags::empty(),
                value: Value::Object(props_inner),
            },
        ],
    };
    
    let mut cursor = std::io::Cursor::new(&mut buffer[..]);
    PodSerializer::serialize(&mut cursor, &Value::Object(route_object))
        .map_err(|e| ApiError::Internal(format!("Failed to serialize Route: {}", e)))?;
    
    let written = cursor.position() as usize;
    let pod = libspa::pod::Pod::from_bytes(&buffer[..written])
        .ok_or_else(|| ApiError::Internal("Failed to create Pod from serialized data".to_string()))?;
    
    // Set the Route parameter
    let device_borrow = device_ref.borrow();
    if let Some(device) = device_borrow.as_ref() {
        device.set_param(ParamType::Route, 0, pod);
        
        // Run mainloop briefly to allow processing
        let set_done = Rc::new(Cell::new(false));
        let set_done_for_timer = set_done.clone();
        let timeout_set = client.mainloop().clone();
        let _timer_set = client.mainloop().loop_().add_timer(move |_| {
            set_done_for_timer.set(true);
            timeout_set.quit();
        });
        _timer_set.update_timer(Some(std::time::Duration::from_millis(200)), None);
        client.mainloop().run();
    }
    drop(device_borrow);
    
    // Simple confirmation response
    let info = DeviceInfo {
        id,
        name: "updated".to_string(),
        properties: HashMap::new(),
        volume: Some(volume),
    };
    Ok(Json(info))
}

// Unified volume API - works for both devices and sinks
pub async fn list_all_volumes(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<Vec<VolumeInfo>>, ApiError> {
    use crate::PipeWireClient;
    use pipewire as pw;
    use std::cell::RefCell;
    use std::rc::Rc;
    use libspa::param::ParamType;
    
    let client = PipeWireClient::new()
        .map_err(|e| ApiError::Internal(format!("Failed to connect to PipeWire: {}", e)))?;
    
    let mut result_volumes = Vec::new();
    
    // Collect devices with Route parameters
    let devices: Rc<RefCell<Vec<(u32, HashMap<String, String>, pw::device::Device)>>> = 
        Rc::new(RefCell::new(Vec::new()));
    let devices_clone = devices.clone();
    
    let registry_for_bind = client.registry().downgrade();
    let _device_listener = client.registry()
        .add_listener_local()
        .global(move |global| {
            if global.type_ == pw::types::ObjectType::Device {
                if let Some(props) = &global.props {
                    if let Some(reg) = registry_for_bind.upgrade() {
                        if let Ok(dev) = reg.bind::<pw::device::Device, _>(&global) {
                            let mut properties = HashMap::new();
                            for (key, value) in props.iter() {
                                properties.insert(key.to_string(), value.to_string());
                            }
                            devices_clone.borrow_mut().push((global.id, properties, dev));
                        }
                    }
                }
            }
        })
        .global_remove(move |_| {})
        .register();
    
    // Collect Audio/Sink nodes
    let sinks: Rc<RefCell<Vec<(u32, HashMap<String, String>, pw::node::Node)>>> = 
        Rc::new(RefCell::new(Vec::new()));
    let sinks_clone = sinks.clone();
    
    let registry_for_nodes = client.registry().downgrade();
    let _sink_listener = client.registry()
        .add_listener_local()
        .global(move |global| {
            if global.type_ == pw::types::ObjectType::Node {
                if let Some(props) = &global.props {
                    if props.get("media.class") == Some("Audio/Sink") {
                        if let Some(reg) = registry_for_nodes.upgrade() {
                            if let Ok(node) = reg.bind::<pw::node::Node, _>(&global) {
                                let mut properties = HashMap::new();
                                for (key, value) in props.iter() {
                                    properties.insert(key.to_string(), value.to_string());
                                }
                                sinks_clone.borrow_mut().push((global.id, properties, node));
                            }
                        }
                    }
                }
            }
        })
        .register();
    
    let timeout_mainloop = client.mainloop().clone();
    let _timer = client.mainloop().loop_().add_timer(move |_| {
        timeout_mainloop.quit();
    });
    _timer.update_timer(Some(std::time::Duration::from_millis(500)), None);
    
    client.mainloop().run();
    
    // Process devices (Route parameters with channelVolumes)
    for (device_id, properties, device) in devices.borrow().iter() {
        let name = properties.get("device.name")
            .or_else(|| properties.get("device.description"))
            .map(|s| s.as_str())
            .unwrap_or("unknown");
        
        let volume_ref: Rc<RefCell<Option<f32>>> = Rc::new(RefCell::new(None));
        let volume_ref_clone = volume_ref.clone();
        
        let mainloop_for_param = client.mainloop().clone();
        let _param_listener = device
            .add_listener_local()
            .param(move |_, param_type, _, _, param_pod| {
                if param_type != ParamType::Route {
                    return;
                }
                
                if let Some(pod) = param_pod {
                    let parsed = crate::pod_parser::parse_props_pod(pod);
                    
                    if let Some(JsonValue::Array(volumes)) = parsed.get("channelVolumes") {
                        if let Some(JsonValue::Number(vol)) = volumes.first() {
                            if let Some(v) = vol.as_f64() {
                                *volume_ref_clone.borrow_mut() = Some(v as f32);
                            }
                        }
                    } else if let Some(JsonValue::Object(props_obj)) = parsed.get("prop_10") {
                        if let Some(JsonValue::Array(volumes)) = props_obj.get("channelVolumes") {
                            if let Some(JsonValue::Number(vol)) = volumes.first() {
                                if let Some(v) = vol.as_f64() {
                                    *volume_ref_clone.borrow_mut() = Some(v as f32);
                                }
                            }
                        }
                    }
                }
                
                mainloop_for_param.quit();
            })
            .register();
        
        let timeout_mainloop2 = client.mainloop().clone();
        let _timer2 = client.mainloop().loop_().add_timer(move |_| {
            timeout_mainloop2.quit();
        });
        _timer2.update_timer(Some(std::time::Duration::from_millis(200)), None);
        
        device.enum_params(0, Some(ParamType::Route), 0, u32::MAX);
        client.mainloop().run();
        
        // Only include devices that have a volume
        let vol = *volume_ref.borrow();
        if let Some(vol) = vol {
            let volume_info = VolumeInfo {
                id: *device_id,
                name: name.to_string(),
                object_type: "device".to_string(),
                volume: Some(vol),
            };
            
            result_volumes.push(volume_info);
        }
    }
    
    // Process sinks (Props parameters with volume)
    for (sink_id, properties, node) in sinks.borrow().iter() {
        let name = properties.get("node.name")
            .or_else(|| properties.get("node.description"))
            .map(|s| s.as_str())
            .unwrap_or("unknown");
        
        let volume_ref: Rc<RefCell<Option<f32>>> = Rc::new(RefCell::new(None));
        let volume_ref_clone = volume_ref.clone();
        
        let mainloop_for_param = client.mainloop().clone();
        let _param_listener = node
            .add_listener_local()
            .param(move |_, param_type, _, _, param_pod| {
                if param_type != ParamType::Props {
                    return;
                }
                
                if let Some(pod) = param_pod {
                    let parsed = crate::pod_parser::parse_props_pod(pod);
                    
                    // Look for volume property (65539)
                    if let Some(JsonValue::Number(vol)) = parsed.get("volume") {
                        if let Some(v) = vol.as_f64() {
                            *volume_ref_clone.borrow_mut() = Some(v as f32);
                        }
                    }
                }
                
                mainloop_for_param.quit();
            })
            .register();
        
        let timeout_mainloop3 = client.mainloop().clone();
        let _timer3 = client.mainloop().loop_().add_timer(move |_| {
            timeout_mainloop3.quit();
        });
        _timer3.update_timer(Some(std::time::Duration::from_millis(200)), None);
        
        node.enum_params(0, Some(ParamType::Props), 0, u32::MAX);
        client.mainloop().run();
        
        // Only include sinks that have a volume
        let vol = *volume_ref.borrow();
        if let Some(vol) = vol {
            let volume_info = VolumeInfo {
                id: *sink_id,
                name: name.to_string(),
                object_type: "sink".to_string(),
                volume: Some(vol),
            };
            
            result_volumes.push(volume_info);
        }
    }
    
    Ok(Json(result_volumes))
}

// Get volume for a specific ID (auto-detects device or sink)
pub async fn get_volume_by_id(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> Result<Json<VolumeInfo>, ApiError> {
    use crate::PipeWireClient;
    use pipewire as pw;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;
    use libspa::param::ParamType;
    
    let client = PipeWireClient::new()
        .map_err(|e| ApiError::Internal(format!("Failed to connect to PipeWire: {}", e)))?;
    
    // Store object info and bound device/node in one pass
    let device_ref: Rc<RefCell<Option<pw::device::Device>>> = Rc::new(RefCell::new(None));
    let node_ref: Rc<RefCell<Option<pw::node::Node>>> = Rc::new(RefCell::new(None));
    let object_info: Rc<RefCell<Option<(String, String)>>> = Rc::new(RefCell::new(None)); // (obj_type, name)
    
    let device_ref_clone = device_ref.clone();
    let node_ref_clone = node_ref.clone();
    let object_info_clone = object_info.clone();
    
    let done = Rc::new(Cell::new(false));
    let done_clone = done.clone();
    let mainloop_clone = client.mainloop().clone();
    let registry_weak = client.registry().downgrade();
    
    // Find the object and bind to it in one pass
    let _listener = client.registry()
        .add_listener_local()
        .global(move |global| {
            if global.id == id {
                if let Some(props) = &global.props {
                    if global.type_ == pw::types::ObjectType::Device {
                        let name = props.get("device.name")
                            .or_else(|| props.get("device.description"))
                            .unwrap_or("unknown")
                            .to_string();
                        
                        *object_info_clone.borrow_mut() = Some(("device".to_string(), name));
                        
                        // Bind to device
                        if let Some(reg) = registry_weak.upgrade() {
                            if let Ok(dev) = reg.bind::<pw::device::Device, _>(&global) {
                                *device_ref_clone.borrow_mut() = Some(dev);
                            }
                        }
                        done_clone.set(true);
                        mainloop_clone.quit();
                    } else if global.type_ == pw::types::ObjectType::Node {
                        if props.get("media.class") == Some("Audio/Sink") {
                            let name = props.get("node.name")
                                .or_else(|| props.get("node.description"))
                                .unwrap_or("unknown")
                                .to_string();
                            
                            *object_info_clone.borrow_mut() = Some(("sink".to_string(), name));
                            
                            // Bind to node
                            if let Some(reg) = registry_weak.upgrade() {
                                if let Ok(node) = reg.bind::<pw::node::Node, _>(&global) {
                                    *node_ref_clone.borrow_mut() = Some(node);
                                }
                            }
                            done_clone.set(true);
                            mainloop_clone.quit();
                        }
                    }
                }
            }
        })
        .register();
    
    let timeout_mainloop = client.mainloop().clone();
    let timeout_done = done.clone();
    let _timer = client.mainloop().loop_().add_timer(move |_| {
        if !timeout_done.get() {
            timeout_mainloop.quit();
        }
    });
    _timer.update_timer(Some(std::time::Duration::from_millis(500)), None);
    
    client.mainloop().run();
    
    if !done.get() {
        return Err(ApiError::NotFound(format!("Object {} not found or not a volume-capable object", id)));
    }
    
    let (obj_type, name) = object_info.borrow().clone()
        .ok_or_else(|| ApiError::Internal("Failed to get object info".to_string()))?;
    
    // Now get the volume based on object type
    let volume = if obj_type == "device" {
        let volume_ref: Rc<RefCell<Option<f32>>> = Rc::new(RefCell::new(None));
        let volume_ref_clone = volume_ref.clone();
        
        let device_borrow = device_ref.borrow();
        if let Some(device) = device_borrow.as_ref() {
            let mainloop_for_param = client.mainloop().clone();
            let _param_listener = device
                .add_listener_local()
                .param(move |_, param_type, _, _, param_pod| {
                    if param_type != ParamType::Route {
                        return;
                    }
                    
                    if let Some(pod) = param_pod {
                        let parsed = crate::pod_parser::parse_props_pod(pod);
                        
                        if let Some(JsonValue::Array(volumes)) = parsed.get("channelVolumes") {
                            if let Some(JsonValue::Number(vol)) = volumes.first() {
                                if let Some(v) = vol.as_f64() {
                                    *volume_ref_clone.borrow_mut() = Some(v as f32);
                                }
                            }
                        } else if let Some(JsonValue::Object(props_obj)) = parsed.get("prop_10") {
                            if let Some(JsonValue::Array(volumes)) = props_obj.get("channelVolumes") {
                                if let Some(JsonValue::Number(vol)) = volumes.first() {
                                    if let Some(v) = vol.as_f64() {
                                        *volume_ref_clone.borrow_mut() = Some(v as f32);
                                    }
                                }
                            }
                        }
                    }
                    
                    mainloop_for_param.quit();
                })
                .register();
            
            let timeout_param = client.mainloop().clone();
            let _timer_param = client.mainloop().loop_().add_timer(move |_| {
                timeout_param.quit();
            });
            _timer_param.update_timer(Some(std::time::Duration::from_millis(200)), None);
            
            device.enum_params(0, Some(ParamType::Route), 0, u32::MAX);
            client.mainloop().run();
        }
        
        let vol = *volume_ref.borrow();
        vol
    } else {
        // Use Props parameters for sink
        let volume_ref: Rc<RefCell<Option<f32>>> = Rc::new(RefCell::new(None));
        let volume_ref_clone = volume_ref.clone();
        
        let node_borrow = node_ref.borrow();
        if let Some(node) = node_borrow.as_ref() {
            let mainloop_for_param = client.mainloop().clone();
            let _param_listener = node
                .add_listener_local()
                .param(move |_, param_type, _, _, param_pod| {
                    if param_type != ParamType::Props {
                        return;
                    }
                    
                    if let Some(pod) = param_pod {
                        let parsed = crate::pod_parser::parse_props_pod(pod);
                        
                        if let Some(JsonValue::Number(vol)) = parsed.get("volume") {
                            if let Some(v) = vol.as_f64() {
                                *volume_ref_clone.borrow_mut() = Some(v as f32);
                            }
                        }
                    }
                    
                    mainloop_for_param.quit();
                })
                .register();
            
            let timeout_param = client.mainloop().clone();
            let _timer_param = client.mainloop().loop_().add_timer(move |_| {
                timeout_param.quit();
            });
            _timer_param.update_timer(Some(std::time::Duration::from_millis(200)), None);
            
            node.enum_params(0, Some(ParamType::Props), 0, u32::MAX);
            client.mainloop().run();
        }
        
        let vol = *volume_ref.borrow();
        vol
    };
    
    let info = VolumeInfo {
        id,
        name,
        object_type: obj_type,
        volume,
    };
    
    Ok(Json(info))
}

// Set volume for a specific ID (auto-detects device or sink)
pub async fn set_volume_by_id(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<u32>,
    Json(request): Json<SetVolumeRequest>,
) -> Result<Json<VolumeResponse>, ApiError> {
    use crate::PipeWireClient;
    use pipewire as pw;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;
    use libspa::param::ParamType;
    
    let volume = request.volume;
    
    let client = PipeWireClient::new()
        .map_err(|e| ApiError::Internal(format!("Failed to connect to PipeWire: {}", e)))?;
    
    let object_type: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let object_type_clone = object_type.clone();
    
    let done = Rc::new(Cell::new(false));
    let done_clone = done.clone();
    let mainloop_clone = client.mainloop().clone();
    
    // Detect object type
    let _listener = client.registry()
        .add_listener_local()
        .global(move |global| {
            if global.id == id {
                if global.type_ == pw::types::ObjectType::Device {
                    *object_type_clone.borrow_mut() = Some("device".to_string());
                    done_clone.set(true);
                    mainloop_clone.quit();
                } else if global.type_ == pw::types::ObjectType::Node {
                    if let Some(props) = &global.props {
                        if props.get("media.class") == Some("Audio/Sink") {
                            *object_type_clone.borrow_mut() = Some("sink".to_string());
                            done_clone.set(true);
                            mainloop_clone.quit();
                        }
                    }
                }
            }
        })
        .register();
    
    let timeout_mainloop = client.mainloop().clone();
    let _timer = client.mainloop().loop_().add_timer(move |_| {
        timeout_mainloop.quit();
    });
    _timer.update_timer(Some(std::time::Duration::from_millis(500)), None);
    
    client.mainloop().run();
    
    let obj_type = object_type.borrow().clone()
        .ok_or_else(|| ApiError::NotFound(format!("Object {} not found or not a volume-capable object", id)))?;
    
    if obj_type == "device" {
        // Set volume via Route parameters (channelVolumes)
        let device_ref: Rc<RefCell<Option<pw::device::Device>>> = Rc::new(RefCell::new(None));
        let device_ref_clone = device_ref.clone();
        let device_ref_for_set = device_ref.clone();
        
        let registry_for_bind = client.registry().downgrade();
        let _bind_listener = client.registry()
            .add_listener_local()
            .global(move |global| {
                if global.id == id && global.type_ == pw::types::ObjectType::Device {
                    if let Some(reg) = registry_for_bind.upgrade() {
                        if let Ok(dev) = reg.bind::<pw::device::Device, _>(&global) {
                            *device_ref_clone.borrow_mut() = Some(dev);
                        }
                    }
                }
            })
            .register();
        
        let timeout_bind = client.mainloop().clone();
        let _timer_bind = client.mainloop().loop_().add_timer(move |_| {
            timeout_bind.quit();
        });
        _timer_bind.update_timer(Some(std::time::Duration::from_millis(200)), None);
        client.mainloop().run();
        
        let device_borrow = device_ref_for_set.borrow();
        if let Some(device) = device_borrow.as_ref() {
            // Build Route Pod with channelVolumes (similar to set_device_volume)
            use libspa::pod::serialize::PodSerializer;
            use libspa::pod::{Object, Property, Value};
            
            let mut buffer = vec![0u8; 1024];
            
            let props_inner = Object {
                type_: 262146, // SPA_TYPE_OBJECT_ParamProps
                id: libspa::sys::SPA_PARAM_Props,
                properties: vec![
                    Property {
                        key: 65544, // channelVolumes
                        flags: libspa::pod::PropertyFlags::empty(),
                        value: Value::ValueArray(libspa::pod::ValueArray::Float(vec![volume, volume])),
                    },
                    Property {
                        key: 65547, // channelMap
                        flags: libspa::pod::PropertyFlags::empty(),
                        value: Value::ValueArray(libspa::pod::ValueArray::Id(vec![
                            libspa::utils::Id(3), // FL
                            libspa::utils::Id(4), // FR
                        ])),
                    },
                ],
            };
            
            let route_object = Object {
                type_: 262153, // SPA_TYPE_OBJECT_ParamRoute
                id: libspa::sys::SPA_PARAM_Route,
                properties: vec![
                    Property {
                        key: 1, // index
                        flags: libspa::pod::PropertyFlags::empty(),
                        value: Value::Int(0),
                    },
                    Property {
                        key: 2, // direction
                        flags: libspa::pod::PropertyFlags::empty(),
                        value: Value::Id(libspa::utils::Id(1)),
                    },
                    Property {
                        key: 3, // device
                        flags: libspa::pod::PropertyFlags::empty(),
                        value: Value::Int(1),
                    },
                    Property {
                        key: 10, // props
                        flags: libspa::pod::PropertyFlags::empty(),
                        value: Value::Object(props_inner),
                    },
                ],
            };
            
            let mut cursor = std::io::Cursor::new(&mut buffer[..]);
            PodSerializer::serialize(&mut cursor, &Value::Object(route_object))
                .map_err(|e| ApiError::Internal(format!("Failed to serialize Route: {}", e)))?;
            
            let written = cursor.position() as usize;
            let pod = libspa::pod::Pod::from_bytes(&buffer[..written])
                .ok_or_else(|| ApiError::Internal("Failed to create Pod from serialized data".to_string()))?;
            
            device.set_param(ParamType::Route, 0, pod);
            
            let timeout_set = client.mainloop().clone();
            let _timer_set = client.mainloop().loop_().add_timer(move |_| {
                timeout_set.quit();
            });
            _timer_set.update_timer(Some(std::time::Duration::from_millis(200)), None);
            client.mainloop().run();
        }
    } else {
        // Set volume via Props parameters (volume property)
        let node_ref: Rc<RefCell<Option<pw::node::Node>>> = Rc::new(RefCell::new(None));
        let node_ref_clone = node_ref.clone();
        let node_ref_for_set = node_ref.clone();
        
        let registry_for_bind = client.registry().downgrade();
        let _bind_listener = client.registry()
            .add_listener_local()
            .global(move |global| {
                if global.id == id && global.type_ == pw::types::ObjectType::Node {
                    if let Some(reg) = registry_for_bind.upgrade() {
                        if let Ok(node) = reg.bind::<pw::node::Node, _>(&global) {
                            *node_ref_clone.borrow_mut() = Some(node);
                        }
                    }
                }
            })
            .register();
        
        let timeout_bind = client.mainloop().clone();
        let _timer_bind = client.mainloop().loop_().add_timer(move |_| {
            timeout_bind.quit();
        });
        _timer_bind.update_timer(Some(std::time::Duration::from_millis(200)), None);
        client.mainloop().run();
        
        let node_borrow = node_ref_for_set.borrow();
        if let Some(node) = node_borrow.as_ref() {
            // Build Props Pod with volume
            use libspa::pod::serialize::PodSerializer;
            use libspa::pod::{Object, Property, Value};
            
            let mut buffer = vec![0u8; 1024];
            
            let values = vec![
                Property {
                    key: 65539, // SPA_PROP_volume
                    flags: libspa::pod::PropertyFlags::empty(),
                    value: Value::Float(volume),
                },
            ];
            
            let obj = Object {
                type_: 262146, // SPA_TYPE_OBJECT_ParamProps
                id: libspa::sys::SPA_PARAM_Props,
                properties: values,
            };
            
            let mut cursor = std::io::Cursor::new(&mut buffer[..]);
            PodSerializer::serialize(&mut cursor, &Value::Object(obj))
                .map_err(|e| ApiError::Internal(format!("Failed to serialize Pod: {}", e)))?;
            
            let written = cursor.position() as usize;
            let pod = libspa::pod::Pod::from_bytes(&buffer[..written])
                .ok_or_else(|| ApiError::Internal("Failed to create Pod from serialized data".to_string()))?;
            
            node.set_param(ParamType::Props, 0, pod);
            
            let timeout_set = client.mainloop().clone();
            let _timer_set = client.mainloop().loop_().add_timer(move |_| {
                timeout_set.quit();
            });
            _timer_set.update_timer(Some(std::time::Duration::from_millis(200)), None);
            client.mainloop().run();
        }
    }
    
    Ok(Json(VolumeResponse { volume: Some(volume) }))
}

/// Save all current volumes to state file
pub async fn save_all_volumes(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Get all current volumes
    let volumes = list_all_volumes(State(_state.clone())).await?;
    
    // Convert to state format
    let states: Vec<crate::config::VolumeState> = volumes.0
        .into_iter()
        .filter_map(|v| {
            v.volume.map(|vol| crate::config::VolumeState {
                name: v.name,
                volume: vol,
            })
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
    let volume_info = get_volume_by_id(State(_state), Path(id)).await?;
    
    if let Some(volume) = volume_info.volume {
        // Save to state file using name
        crate::config::save_single_volume_state(volume_info.name.clone(), volume)
            .map_err(|e| ApiError::Internal(format!("Failed to save volume state: {}", e)))?;
        
        Ok(Json(serde_json::json!({
            "success": true,
            "id": id,
            "name": volume_info.name,
            "volume": volume,
            "message": "Volume state saved"
        })))
    } else {
        Err(ApiError::NotFound(format!("Volume not available for object {}", id)))
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
        .route("/api/v1/devices", get(list_devices_with_info))
        .route("/api/v1/devices/:id", get(get_device_info))
        .route("/api/v1/devices/:id/volume", get(get_device_volume))
        .route("/api/v1/devices/:id/volume", put(set_device_volume))
        .route("/api/v1/volume", get(list_all_volumes))
        .route("/api/v1/volume/:id", get(get_volume_by_id))
        .route("/api/v1/volume/:id", put(set_volume_by_id))
        .route("/api/v1/volume/save", post(save_all_volumes))
        .route("/api/v1/volume/save/:id", post(save_volume))
        .with_state(state)
}
