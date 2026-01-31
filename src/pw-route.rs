use anyhow::{anyhow, Result};
use clap::Parser;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use pipewire as pw;
use pw::spa::param::ParamType;

#[derive(Parser, Debug)]
#[command(name = "pw-route")]
#[command(about = "Read and modify PipeWire device routes", long_about = None)]
struct Args {
    /// Device ID to query or modify
    device_id: u32,
    
    /// Set route volume (linear 0.0-1.0)
    #[arg(long)]
    set_volume: Option<f32>,
    
    /// Route index (default: 0)
    #[arg(long, default_value_t = 0)]
    route_index: i32,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize PipeWire
    pw::init();

    let mainloop = pw::main_loop::MainLoopRc::new(None)?;
    let context = pw::context::ContextRc::new(&mainloop, None)?;
    let core = context.connect_rc(None)?;
    let registry = core.get_registry_rc()?;

    // Track whether we found the device
    let device_ref: Rc<RefCell<Option<pw::device::Device>>> = Rc::new(RefCell::new(None));
    let device_ref_clone = device_ref.clone();
    
    let done = Rc::new(Cell::new(false));
    let done_clone = done.clone();
    let mainloop_clone = mainloop.clone();

    // Listen for the device
    let target_id = args.device_id;
    let registry_for_bind = registry.downgrade();
    let _listener = registry
        .add_listener_local()
        .global(move |global| {
            if global.id == target_id && global.type_ == pw::types::ObjectType::Device {
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
        return Err(anyhow!("Device {} not found", args.device_id));
    }

    let device_borrow = device_ref.borrow();
    let device = device_borrow.as_ref().unwrap();

    // If setting volume, do it now
    if let Some(volume) = args.set_volume {
        println!("Setting route {} volume to {} on device {}", args.route_index, volume, args.device_id);
        
        // Build Route parameter with updated volume
        use libspa::pod::{serialize::PodSerializer, Object, Property, Value};
        
        let mut buffer = vec![0u8; 4096];
        
        // Create the nested Props object with volume parameters
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
        
        // Create the Route object
        let route_object = Object {
            type_: 262153, // SPA_TYPE_OBJECT_ParamRoute
            id: libspa::sys::SPA_PARAM_Route,
            properties: vec![
                Property {
                    key: 1, // index
                    flags: libspa::pod::PropertyFlags::empty(),
                    value: Value::Int(args.route_index),
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
            .map_err(|e| anyhow!("Failed to serialize Route: {}", e))?;
        
        let written = cursor.position() as usize;
        let pod = libspa::pod::Pod::from_bytes(&buffer[..written])
            .ok_or_else(|| anyhow!("Failed to create Pod from serialized data"))?;
        
        // Set the Route parameter on the device
        device.set_param(ParamType::Route, 0, pod);
        
        println!("Route volume set successfully");
        
        // Run mainloop briefly to allow processing
        let set_done = Rc::new(Cell::new(false));
        let set_done_for_timer = set_done.clone();
        let timeout_set = mainloop.clone();
        let _timer_set = mainloop.loop_().add_timer(move |_| {
            set_done_for_timer.set(true);
            timeout_set.quit();
        });
        _timer_set.update_timer(Some(std::time::Duration::from_millis(200)), None);
        mainloop.run();
        
    } else {
        // Just display route info
        println!("Device ID: {}", args.device_id);
        println!("\nRoute Parameters:");
        
        // Enumerate Route parameters
        let routes_map: Rc<RefCell<Vec<serde_json::Value>>> = Rc::new(RefCell::new(Vec::new()));
        let routes_map_clone = routes_map.clone();
        
        let param_done = Rc::new(Cell::new(false));
        let param_done_for_timer = param_done.clone();
        let param_done_for_listener = param_done.clone();
        
        let timeout_mainloop2 = mainloop.clone();
        let _timer2 = mainloop.loop_().add_timer(move |_| {
            if !param_done_for_timer.get() {
                timeout_mainloop2.quit();
            }
        });
        _timer2.update_timer(Some(std::time::Duration::from_millis(500)), None);
        
        let mainloop_for_param = mainloop.clone();
        let _param_listener = device
            .add_listener_local()
            .param(move |_, param_type, _, _, param_pod| {
                if param_type != ParamType::Route {
                    return;
                }
                
                if let Some(pod) = param_pod {
                    let parsed = pw_api::pod_parser::parse_props_pod(pod);
                    routes_map_clone.borrow_mut().push(serde_json::json!(parsed));
                }
                
                param_done_for_listener.set(true);
                mainloop_for_param.quit();
            })
            .register();
        
        device.enum_params(0, Some(ParamType::Route), 0, u32::MAX);
        mainloop.run();
        
        let routes = routes_map.borrow();
        if routes.is_empty() {
            println!("  (no routes found)");
        } else {
            for (idx, route) in routes.iter().enumerate() {
                println!("\nRoute {}:", idx);
                println!("{}", serde_json::to_string_pretty(route)?);
            }
        }
    }

    Ok(())
}
