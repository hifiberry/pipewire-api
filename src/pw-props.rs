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
