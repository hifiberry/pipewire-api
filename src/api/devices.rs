//! Device API handlers - device info and device-specific volume control
//! 
//! Note: For general volume control, use the unified volume API instead.

use axum::{
    extract::{Path, State},
    Json,
};
use std::sync::Arc;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::collections::HashMap;
use serde_json::Value as JsonValue;
use libspa::param::ParamType;

use crate::api_server::{ApiError, AppState};
use crate::PipeWireClient;
use super::types::*;

/// List all devices with volume information
pub async fn list_devices_with_info(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<Vec<DeviceInfo>>, ApiError> {
    use pipewire as pw;
    
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

/// Get device info including volume from Route parameters
pub async fn get_device_info(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> Result<Json<DeviceInfo>, ApiError> {
    use pipewire as pw;
    
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

/// Get device volume only
pub async fn get_device_volume(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> Result<Json<VolumeResponse>, ApiError> {
    use pipewire as pw;
    
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

/// Set device volume via Route parameters
/// 
/// Note: This uses direct PipeWire API which may not work reliably.
/// Consider using the unified volume API (/api/v1/volume) instead.
pub async fn set_device_volume(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<u32>,
    Json(request): Json<SetVolumeRequest>,
) -> Result<Json<DeviceInfo>, ApiError> {
    use pipewire as pw;
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
