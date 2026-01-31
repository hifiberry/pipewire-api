use anyhow::Result;
use pipewire as pw;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use regex::Regex;
use std::collections::HashMap;
use libspa::param::ParamType;
use libspa::pod::{serialize::PodSerializer, Object, Property, Value};
use tracing::{debug, info, warn, error};

use crate::config::DeviceVolumeRule;
use crate::PipeWireClient;

/// Apply device volume rules on startup
pub fn apply_device_volumes(rules: Vec<DeviceVolumeRule>) -> Result<()> {
    if rules.is_empty() {
        info!("No device volume rules to apply");
        return Ok(());
    }
    
    info!("Applying {} device volume rule(s)", rules.len());
    
    let client = PipeWireClient::new()?;
    
    // Collect all devices
    let devices: Rc<RefCell<Vec<(u32, HashMap<String, String>, pw::device::Device)>>> = 
        Rc::new(RefCell::new(Vec::new()));
    let devices_clone = devices.clone();
    
    let _done = Rc::new(Cell::new(false));
    
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
        .register();
    
    // Set up timeout
    let timeout_mainloop = client.mainloop().clone();
    let _timer = client.mainloop().loop_().add_timer(move |_| {
        timeout_mainloop.quit();
    });
    _timer.update_timer(Some(std::time::Duration::from_secs(2)), None);
    
    client.mainloop().run();
    
    let collected_devices = devices.borrow();
    info!("Found {} device(s)", collected_devices.len());
    
    // Apply rules to matching devices
    for rule in &rules {
        debug!("Processing rule: {}", rule.name);
        
        // Compile regex patterns
        let mut regex_patterns: HashMap<String, Regex> = HashMap::new();
        for (key, pattern) in &rule.device {
            match Regex::new(pattern) {
                Ok(re) => {
                    regex_patterns.insert(key.clone(), re);
                }
                Err(e) => {
                    warn!("Invalid regex pattern '{}' in rule '{}': {}", pattern, rule.name, e);
                    continue;
                }
            }
        }
        
        // Find matching devices
        for (device_id, props, device) in collected_devices.iter() {
            let mut matches = true;
            
            for (key, regex) in &regex_patterns {
                if let Some(value) = props.get(key) {
                    if !regex.is_match(value) {
                        matches = false;
                        break;
                    }
                } else {
                    matches = false;
                    break;
                }
            }
            
            if matches {
                let device_name = props.get("device.name")
                    .or_else(|| props.get("device.description"))
                    .map(|s| s.as_str())
                    .unwrap_or("unknown");
                    
                info!("Applying volume {} to device {} ({})", rule.volume, device_id, device_name);
                
                if let Err(e) = set_device_volume(device, rule.volume) {
                    error!("Failed to set volume for device {}: {}", device_id, e);
                } else {
                    debug!("Successfully set volume for device {}", device_id);
                }
            }
        }
    }
    
    // Run mainloop briefly to process changes
    let process_done = Rc::new(Cell::new(false));
    let process_done_for_timer = process_done.clone();
    let timeout_process = client.mainloop().clone();
    let _timer_process = client.mainloop().loop_().add_timer(move |_| {
        process_done_for_timer.set(true);
        timeout_process.quit();
    });
    _timer_process.update_timer(Some(std::time::Duration::from_millis(500)), None);
    client.mainloop().run();
    
    Ok(())
}

/// Set volume on a device via Route parameters
fn set_device_volume(device: &pw::device::Device, volume: f32) -> Result<()> {
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
    PodSerializer::serialize(&mut cursor, &Value::Object(route_object))?;
    
    let written = cursor.position() as usize;
    let pod = libspa::pod::Pod::from_bytes(&buffer[..written])
        .ok_or_else(|| anyhow::anyhow!("Failed to create Pod from serialized data"))?;
    
    device.set_param(ParamType::Route, 0, pod);
    
    Ok(())
}
