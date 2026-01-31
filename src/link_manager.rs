use anyhow::{anyhow, Result};
use pipewire as pw;
use pipewire::proxy::ProxyT;
use std::cell::RefCell;
use std::rc::Rc;

use crate::linker::{LinkRule, LinkType, NodeIdentifier};

/// Information about a node with its properties
#[derive(Debug, Clone)]
struct NodeInfo {
    id: u32,
    node_name: Option<String>,
    node_nick: Option<String>,
    object_path: Option<String>,
}

/// Information about a port
#[derive(Debug, Clone)]
struct PortInfo {
    id: u32,
    node_id: u32,
    name: String,
    is_output: bool,
}

/// Information about an existing link
#[derive(Debug, Clone)]
struct LinkInfo {
    id: u32,
    output_port: u32,
    input_port: u32,
}

/// Result of applying a link rule
#[derive(Debug, Clone)]
pub struct LinkRuleResult {
    pub success: bool,
    pub message: String,
}

/// Check if a node matches an identifier
fn matches_identifier(node: &NodeInfo, identifier: &NodeIdentifier) -> bool {
    use regex::Regex;
    
    let regex_match = |pattern: &str, text: &str| -> bool {
        if let Ok(re) = Regex::new(pattern) {
            re.is_match(text)
        } else {
            false
        }
    };
    
    if let Some(ref pattern) = identifier.node_name {
        if let Some(ref name) = node.node_name {
            if regex_match(pattern, name) {
                return true;
            }
        }
    }
    
    if let Some(ref pattern) = identifier.node_nick {
        if let Some(ref nick) = node.node_nick {
            if regex_match(pattern, nick) {
                return true;
            }
        }
    }
    
    if let Some(ref pattern) = identifier.object_path {
        if let Some(ref path) = node.object_path {
            if regex_match(pattern, path) {
                return true;
            }
        }
    }
    
    false
}

/// Create a link between two ports
fn create_port_link(
    core: &pw::core::CoreRc,
    output_port_id: u32,
    input_port_id: u32,
) -> Result<u32> {
    // Create link properties using the properties! macro from the properties module
    let props = pw::properties::properties! {
        "link.output.port" => output_port_id.to_string(),
        "link.input.port" => input_port_id.to_string(),
    };
    
    // Create the link using the core's create_object method
    // The type string for links is "link-factory"
    let proxy = core.create_object::<pw::link::Link>(
        "link-factory",
        &props,
    )?;
    
    // Get the proxy ID
    let link_id = proxy.upcast_ref().id();
    
    Ok(link_id)
}

/// Apply a link rule and return results
pub fn apply_link_rule(
    registry: &pw::registry::RegistryRc,
    core: &pw::core::CoreRc,
    mainloop: &pw::main_loop::MainLoopRc,
    rule: &LinkRule,
) -> Result<Vec<LinkRuleResult>> {
    let mut results = Vec::new();
    
    // Store created link proxies to keep them alive
    let link_proxies: Rc<RefCell<Vec<pw::link::Link>>> = Rc::new(RefCell::new(Vec::new()));
    let link_proxies_clone = link_proxies.clone();
    
    // Collect ALL nodes, ports, and existing links in a single pass
    let all_nodes: Rc<RefCell<Vec<NodeInfo>>> = Rc::new(RefCell::new(Vec::new()));
    let all_nodes_clone = all_nodes.clone();
    
    let all_ports: Rc<RefCell<Vec<PortInfo>>> = Rc::new(RefCell::new(Vec::new()));
    let all_ports_clone = all_ports.clone();
    
    let existing_links: Rc<RefCell<Vec<LinkInfo>>> = Rc::new(RefCell::new(Vec::new()));
    let existing_links_clone = existing_links.clone();
    
    // Set up timeout
    let timeout_mainloop = mainloop.clone();
    let _timer = mainloop.loop_().add_timer(move |_| {
        timeout_mainloop.quit();
    });
    _timer.update_timer(Some(std::time::Duration::from_secs(2)), None);
    
    let _listener = registry
        .add_listener_local()
        .global({
            move |global| {
                if global.type_ == pw::types::ObjectType::Node {
                    if let Some(props) = &global.props {
                        all_nodes_clone.borrow_mut().push(NodeInfo {
                            id: global.id,
                            node_name: props.get("node.name").map(|s| s.to_string()),
                            node_nick: props.get("node.nick").map(|s| s.to_string()),
                            object_path: props.get("object.path").map(|s| s.to_string()),
                        });
                    }
                } else if global.type_ == pw::types::ObjectType::Port {
                    if let Some(props) = &global.props {
                        if let Some(node_id_str) = props.get("node.id") {
                            if let Ok(node_id) = node_id_str.parse::<u32>() {
                                let port_name = props.get("port.name")
                                    .or_else(|| props.get("port.alias"))
                                    .unwrap_or("unknown")
                                    .to_string();
                                
                                let is_output = props.get("port.direction")
                                    .map(|d| d == "out")
                                    .unwrap_or(false);
                                
                                all_ports_clone.borrow_mut().push(PortInfo {
                                    id: global.id,
                                    node_id,
                                    name: port_name,
                                    is_output,
                                });
                            }
                        }
                    }
                } else if global.type_ == pw::types::ObjectType::Link {
                    if let Some(props) = &global.props {
                        let output_port = props.get("link.output.port")
                            .and_then(|s| s.parse::<u32>().ok())
                            .unwrap_or(0);
                        let input_port = props.get("link.input.port")
                            .and_then(|s| s.parse::<u32>().ok())
                            .unwrap_or(0);
                        
                        if output_port > 0 && input_port > 0 {
                            existing_links_clone.borrow_mut().push(LinkInfo {
                                id: global.id,
                                output_port,
                                input_port,
                            });
                        }
                    }
                }
            }
        })
        .register();
    
    mainloop.run();
    
    // Filter for source nodes
    let mut sources = Vec::new();
    for node in all_nodes.borrow().iter() {
        if matches_identifier(node, &rule.source) {
            sources.push(node.clone());
        }
    }
    
    if sources.is_empty() {
        return Err(anyhow!("No source nodes found matching criteria"));
    }
    
    // Filter for destination nodes
    let mut destinations = Vec::new();
    for node in all_nodes.borrow().iter() {
        if matches_identifier(node, &rule.destination) {
            destinations.push(node.clone());
        }
    }
    
    if destinations.is_empty() {
        return Err(anyhow!("No destination nodes found matching criteria"));
    }
    
    // Apply the rule for each combination
    match rule.link_type {
        LinkType::Link => {
            for source in &sources {
                for dest in &destinations {
                    let source_name = source.node_name.as_ref()
                        .or(source.node_nick.as_ref())
                        .or(source.object_path.as_ref())
                        .map(|s| s.as_str())
                        .unwrap_or("unknown");
                    
                    let dest_name = dest.node_name.as_ref()
                        .or(dest.node_nick.as_ref())
                        .or(dest.object_path.as_ref())
                        .map(|s| s.as_str())
                        .unwrap_or("unknown");
                    
                    // Find ports for these nodes
                    let mut source_outputs = Vec::new();
                    let mut dest_inputs = Vec::new();
                    
                    for port in all_ports.borrow().iter() {
                        if port.node_id == source.id && port.is_output {
                            source_outputs.push(port.clone());
                        } else if port.node_id == dest.id && !port.is_output {
                            dest_inputs.push(port.clone());
                        }
                    }
                    
                    // Check port counts match
                    if source_outputs.len() != dest_inputs.len() {
                        results.push(LinkRuleResult {
                            success: false,
                            message: format!(
                                "Port count mismatch for {} -> {}: {} output ports vs {} input ports",
                                source_name, dest_name, source_outputs.len(), dest_inputs.len()
                            ),
                        });
                        continue;
                    }
                    
                    if source_outputs.is_empty() {
                        results.push(LinkRuleResult {
                            success: false,
                            message: format!("No ports found to link {} -> {}", source_name, dest_name),
                        });
                        continue;
                    }
                    
                    // Sort ports by ID to ensure consistent ordering
                    source_outputs.sort_by_key(|p| p.id);
                    dest_inputs.sort_by_key(|p| p.id);
                    
                    // Create links for each port pair
                    for (src_port, dst_port) in source_outputs.iter().zip(dest_inputs.iter()) {
                        // Check if this link already exists
                        let link_exists = existing_links.borrow().iter().any(|link| {
                            link.output_port == src_port.id && link.input_port == dst_port.id
                        });
                        
                        if link_exists {
                            results.push(LinkRuleResult {
                                success: true,
                                message: format!(
                                    "Link already exists between port {} ({}) and port {} ({})",
                                    src_port.id, src_port.name, dst_port.id, dst_port.name
                                ),
                            });
                            continue;
                        }
                        
                        match create_port_link(core, src_port.id, dst_port.id) {
                            Ok(link_id) => {
                                // Store the proxy to keep it alive
                                // Add link properties including name and linger
                                let proxy = core.create_object::<pw::link::Link>(
                                    "link-factory",
                                    &pw::properties::properties! {
                                        "link.output.port" => src_port.id.to_string(),
                                        "link.input.port" => dst_port.id.to_string(),
                                        "object.linger" => "true",
                                        "object.name" => rule.name.clone(),
                                    },
                                )?;
                                link_proxies_clone.borrow_mut().push(proxy);
                                
                                results.push(LinkRuleResult {
                                    success: true,
                                    message: format!(
                                        "Created link {} between port {} ({}) and port {} ({})",
                                        link_id, src_port.id, src_port.name, dst_port.id, dst_port.name
                                    ),
                                });
                            }
                            Err(e) => {
                                results.push(LinkRuleResult {
                                    success: false,
                                    message: format!(
                                        "Failed to link port {} ({}) to port {} ({}): {}",
                                        src_port.id, src_port.name, dst_port.id, dst_port.name, e
                                    ),
                                });
                            }
                        }
                    }
                }
            }
        }
        LinkType::Unlink => {
            results.push(LinkRuleResult {
                success: false,
                message: "Unlink operation not yet fully implemented".to_string(),
            });
        }
    }
    
    // Run the mainloop briefly to allow PipeWire to process the link creation
    // Set up a timer to quit the loop after a short delay
    let process_mainloop = mainloop.clone();
    let _timer = mainloop.loop_().add_timer(move |_| {
        process_mainloop.quit();
    });
    _timer.update_timer(Some(std::time::Duration::from_millis(500)), None);
    mainloop.run();
    
    Ok(results)
}
