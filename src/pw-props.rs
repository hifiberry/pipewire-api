use anyhow::{anyhow, Result};
use clap::Parser;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::collections::HashMap;
use pipewire as pw;
use pw::spa::param::ParamType;

#[derive(Parser, Debug)]
#[command(name = "pw-props")]
#[command(about = "List all properties (static and dynamic) for a PipeWire object", long_about = None)]
struct Args {
    /// Object ID to query
    object_id: u32,
    
    /// Set a property value (format: key=value)
    #[arg(short, long)]
    set: Option<String>,
    
    /// Set volume on device route (finds device for node automatically)
    #[arg(long)]
    set_route_volume: Option<f32>,
}

#[derive(Clone)]
struct ObjectInfo {
    id: u32,
    type_: pw::types::ObjectType,
    props: HashMap<String, String>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize PipeWire
    pw::init();

    let mainloop = pw::main_loop::MainLoopRc::new(None)?;
    let context = pw::context::ContextRc::new(&mainloop, None)?;
    let core = context.connect_rc(None)?;
    let registry = core.get_registry_rc()?;

    // Track whether we found the object
    let found_object: Rc<RefCell<Option<ObjectInfo>>> = Rc::new(RefCell::new(None));
    let found_object_clone = Rc::clone(&found_object);
    
    // For node binding - store registry weak ref
    let registry_for_bind = registry.downgrade();
    let node_for_props: Rc<RefCell<Option<pw::node::Node>>> = Rc::new(RefCell::new(None));
    let node_for_props_clone = Rc::clone(&node_for_props);

    let done = Rc::new(Cell::new(false));
    let done_clone = done.clone();
    let mainloop_clone = mainloop.clone();

    // Listen for the specific object
    let target_id = args.object_id;
    let _listener = registry
        .add_listener_local()
        .global(move |global| {
            if global.id == target_id {
                let mut props = HashMap::new();
                if let Some(dict) = &global.props {
                    for (key, value) in dict.iter() {
                        props.insert(key.to_string(), value.to_string());
                    }
                }
                *found_object_clone.borrow_mut() = Some(ObjectInfo {
                    id: global.id,
                    type_: global.type_.clone(),
                    props,
                });
                
                // If it's a node, bind it immediately
                if global.type_ == pw::types::ObjectType::Node {
                    if let Some(reg) = registry_for_bind.upgrade() {
                        if let Ok(n) = reg.bind::<pw::node::Node, _>(&global) {
                            *node_for_props_clone.borrow_mut() = Some(n);
                        }
                    }
                }
                
                done_clone.set(true);
                mainloop_clone.quit();
            }
        })
        .register();

    // Set timeout
    let timeout_mainloop = mainloop.clone();
    let timeout_done = done.clone();
    let _timer = mainloop.loop_().add_timer(move |_| {
        if !timeout_done.get() {
            timeout_mainloop.quit();
        }
    });
    _timer.update_timer(Some(std::time::Duration::from_millis(500)), None);

    mainloop.run();

    if !done.get() {
        return Err(anyhow!("Object {} not found", args.object_id));
    }

    let obj_info = found_object.borrow().clone().unwrap();

    // Check if we need to set a property
    if let Some(set_arg) = &args.set {
        if obj_info.type_ != pw::types::ObjectType::Node {
            return Err(anyhow!("Can only set properties on nodes, object {} is {:?}", args.object_id, obj_info.type_));
        }
        
        let node_borrow = node_for_props.borrow();
        let node = node_borrow.as_ref()
            .ok_or_else(|| anyhow!("Failed to bind to node {}", args.object_id))?;
        
        // Parse key=value
        let parts: Vec<&str> = set_arg.split('=').collect();
        if parts.len() != 2 {
            return Err(anyhow!("Invalid format. Use: key=value"));
        }
        let key = parts[0];
        let value_str = parts[1];
        
        // Try to parse the value as different types
        let pod_value = if value_str.starts_with('[') && value_str.ends_with(']') {
            // Parse array: [val1,val2,...]
            let inner = &value_str[1..value_str.len()-1];
            let values: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
            
            // Try to parse as float array (most common for volumes)
            let floats: Result<Vec<f32>, _> = values.iter().map(|v| v.parse::<f32>()).collect();
            if let Ok(float_vec) = floats {
                libspa::pod::Value::ValueArray(libspa::pod::ValueArray::Float(float_vec))
            } else {
                // Try as int array
                let ints: Result<Vec<i32>, _> = values.iter().map(|v| v.parse::<i32>()).collect();
                if let Ok(int_vec) = ints {
                    libspa::pod::Value::ValueArray(libspa::pod::ValueArray::Int(int_vec))
                } else {
                    return Err(anyhow!("Failed to parse array values"));
                }
            }
        } else if value_str.eq_ignore_ascii_case("true") {
            libspa::pod::Value::Bool(true)
        } else if value_str.eq_ignore_ascii_case("false") {
            libspa::pod::Value::Bool(false)
        } else if let Ok(i) = value_str.parse::<i32>() {
            libspa::pod::Value::Int(i)
        } else if let Ok(f) = value_str.parse::<f32>() {
            libspa::pod::Value::Float(f)
        } else {
            libspa::pod::Value::String(value_str.to_string())
        };
        
        // Map friendly names to property IDs
        let prop_id = match key {
            "volume" => 65539_u32,
            "mute" => 65540_u32,
            "channelVolumes" => 65544_u32,
            "volumeBase" => 65545_u32,
            "volumeStep" => 65546_u32,
            "channelMap" => 65547_u32,
            "monitorMute" => 65548_u32,
            "monitorVolumes" => 65549_u32,
            "softMute" => 65551_u32,
            "softVolumes" => 65552_u32,
            _ => {
                // Try to parse as prop_XXXXX
                if key.starts_with("prop_") {
                    key[5..].parse::<u32>()
                        .map_err(|_| anyhow!("Invalid property key: {}", key))?
                } else {
                    return Err(anyhow!("Unknown property: {}. Use friendly name (volume, mute, etc.) or prop_XXXXX format", key));
                }
            }
        };
        
        // Build the Props object
        use libspa::pod::{serialize::PodSerializer, Object, Property};
        let mut buffer = vec![0u8; 1024];
        let props_object = Object {
            type_: libspa::sys::SPA_TYPE_OBJECT_Props,
            id: libspa::sys::SPA_PARAM_Props,
            properties: vec![Property {
                key: prop_id,
                flags: libspa::pod::PropertyFlags::empty(),
                value: pod_value,
            }],
        };
        
        let mut cursor = std::io::Cursor::new(&mut buffer[..]);
        PodSerializer::serialize(&mut cursor, &libspa::pod::Value::Object(props_object))
            .map_err(|e| anyhow!("Failed to serialize property: {}", e))?;
        
        let written = cursor.position() as usize;
        let pod = libspa::pod::Pod::from_bytes(&buffer[..written])
            .ok_or_else(|| anyhow!("Failed to create Pod from serialized data"))?;
        
        // Set the parameter
        node.set_param(ParamType::Props, 0, pod);
        
        println!("Set property '{}' (id={}) to: {}", key, prop_id, value_str);
        
        // Run mainloop briefly to allow the change to be processed
        let set_done = Rc::new(Cell::new(false));
        let set_done_for_timer = set_done.clone();
        let timeout_mainloop_set = mainloop.clone();
        let _timer_set = mainloop.loop_().add_timer(move |_| {
            set_done_for_timer.set(true);
            timeout_mainloop_set.quit();
        });
        _timer_set.update_timer(Some(std::time::Duration::from_millis(200)), None);
        mainloop.run();
        
        return Ok(());
    }

    // Check if we need to set route volume on a device
    if let Some(volume) = args.set_route_volume {
        // Get device ID from node properties or use object_id if it's a device
        let device_id = if obj_info.type_ == pw::types::ObjectType::Device {
            args.object_id
        } else if obj_info.type_ == pw::types::ObjectType::Node {
            obj_info.props.get("device.id")
                .and_then(|s| s.parse::<u32>().ok())
                .ok_or_else(|| anyhow!("Node {} has no device.id property", args.object_id))?
        } else {
            return Err(anyhow!("Can only set route volume on nodes or devices"));
        };
        
        println!("Setting route volume on device {} to {}", device_id, volume);
        
        // Bind to the device - need to search through all globals
        let device_ref: Rc<RefCell<Option<pw::device::Device>>> = Rc::new(RefCell::new(None));
        let device_ref_clone = device_ref.clone();
        let device_done = Rc::new(Cell::new(false));
        let device_done_clone = device_done.clone();
        let device_mainloop = mainloop.clone();
        
        let registry_for_device = registry.downgrade();
        let _device_listener = registry
            .add_listener_local()
            .global(move |global| {
                if global.id == device_id {
                    if let Some(reg) = registry_for_device.upgrade() {
                        if let Ok(dev) = reg.bind::<pw::device::Device, _>(&global) {
                            *device_ref_clone.borrow_mut() = Some(dev);
                            device_done_clone.set(true);
                            device_mainloop.quit();
                        }
                    }
                }
            })
            .register();
        
        let timeout_device = mainloop.clone();
        let timeout_device_done = device_done.clone();
        let _timer_device = mainloop.loop_().add_timer(move |_| {
            if !timeout_device_done.get() {
                timeout_device.quit();
            }
        });
        _timer_device.update_timer(Some(std::time::Duration::from_secs(5)), None);
        
        mainloop.run();
        
        if !device_done.get() {
            return Err(anyhow!("Device {} not found", device_id));
        }
        
        let device = device_ref.borrow();
        let device = device.as_ref().unwrap();
        
        // Build Route parameter with updated volume
        // The Route object needs index, direction, device, and props with channelVolumes
        use libspa::pod::{serialize::PodSerializer, Object, Property, Value};
        
        let mut buffer = vec![0u8; 2048];
        
        // Create the nested Props object with channelVolumes
        let props_inner = Object {
            type_: libspa::sys::SPA_TYPE_OBJECT_Props,
            id: libspa::sys::SPA_PARAM_Route,
            properties: vec![Property {
                key: 65544, // channelVolumes
                flags: libspa::pod::PropertyFlags::empty(),
                value: Value::ValueArray(libspa::pod::ValueArray::Float(vec![volume, volume])),
            }],
        };
        
        // Create the Route object
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
                Property {
                    key: 13, // save
                    flags: libspa::pod::PropertyFlags::empty(),
                    value: Value::Bool(true),
                },
            ],
        };
        
        let mut cursor = std::io::Cursor::new(&mut buffer[..]);
        PodSerializer::serialize(&mut cursor, &Value::Object(route_object))
            .map_err(|e| anyhow!("Failed to serialize Route: {}", e))?;
        
        let written = cursor.position() as usize;
        let pod = libspa::pod::Pod::from_bytes(&buffer[..written])
            .ok_or_else(|| anyhow!("Failed to create Pod from serialized data"))?;
        
        // Set the Route parameter on the device
        device.set_param(ParamType::Route, 0, pod);
        
        println!("Route volume set successfully");
        
        // Run mainloop to allow processing
        let route_done = Rc::new(Cell::new(false));
        let route_done_for_timer = route_done.clone();
        let timeout_route = mainloop.clone();
        let _timer_route = mainloop.loop_().add_timer(move |_| {
            route_done_for_timer.set(true);
            timeout_route.quit();
        });
        _timer_route.update_timer(Some(std::time::Duration::from_millis(200)), None);
        mainloop.run();
        
        return Ok(());
    }

    // Print static properties
    println!("Object ID: {}", obj_info.id);
    println!("Type: {:?}", obj_info.type_);
    println!("\nStatic Properties:");
    if obj_info.props.is_empty() {
        println!("  (none)");
    } else {
        for (key, value) in obj_info.props.iter() {
            println!("  {}: {}", key, value);
        }
    }

    // If it's a node, try to get dynamic properties
    if obj_info.type_ == pw::types::ObjectType::Node {
        println!("\nDynamic Properties (Props):");
        
        let node_borrow = node_for_props.borrow();
        if let Some(node) = node_borrow.as_ref() {
                let params_map: Rc<RefCell<HashMap<String, serde_json::Value>>> = 
                    Rc::new(RefCell::new(HashMap::new()));
                let params_map_clone = params_map.clone();
                
                let param_done = Rc::new(Cell::new(false));
                let param_done_for_timer = param_done.clone();
                let param_done_for_listener = param_done.clone();
                
                let timeout_mainloop3 = mainloop.clone();
                let _timer3 = mainloop.loop_().add_timer(move |_| {
                    if !param_done_for_timer.get() {
                        timeout_mainloop3.quit();
                    }
                });
                _timer3.update_timer(Some(std::time::Duration::from_millis(500)), None);
                
                let mainloop_for_param = mainloop.clone();
                let _param_listener = node
                    .add_listener_local()
                    .param(move |_, param_type, _, _, param_pod| {
                        if param_type != ParamType::Props {
                            return;
                        }
                        
                        if let Some(pod) = param_pod {
                            let parsed = pw_api::pod_parser::parse_props_pod(pod);
                            params_map_clone.borrow_mut().extend(parsed);
                        }
                        
                        param_done_for_listener.set(true);
                        mainloop_for_param.quit();
                    })
                    .register();
                
                node.enum_params(0, Some(ParamType::Props), 0, u32::MAX);
                mainloop.run();
                
                let params = params_map.borrow();
                if params.is_empty() {
                    println!("  (none)");
                } else {
                    for (key, value) in params.iter() {
                        println!("  {}: {}", key, serde_json::to_string_pretty(value)?);
                    }
                }
        } else {
            println!("  (failed to bind to node)");
        }
    }

    Ok(())
}
