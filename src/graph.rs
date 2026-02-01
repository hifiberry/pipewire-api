//! Graph generation for PipeWire audio topology
//!
//! Generates DOT format graphs of audio nodes and their connections.
//! Filter-chains are combined into single nodes for clarity.

use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use std::collections::{HashMap, HashSet};
use std::process::Command;
use std::sync::Arc;
use tracing::error;

use crate::api_server::AppState;
use crate::pwcli;

/// Represents a combined filter-chain node (input + output merged)
struct FilterChain {
    name: String,
    input_id: u32,
    output_id: u32,
}

/// Check if a node is an audio node (not MIDI or video)
fn is_audio_node(obj: &pwcli::PwObject) -> bool {
    // Check media.class property
    if let Some(media_class) = obj.properties.get("media.class") {
        let class_lower = media_class.to_lowercase();
        // Include audio-related classes
        if class_lower.contains("audio") || class_lower.contains("stream") {
            return true;
        }
        // Exclude MIDI and video
        if class_lower.contains("midi") || class_lower.contains("video") {
            return false;
        }
    }
    
    // Check node.name for known audio patterns
    if let Some(name) = obj.properties.get("node.name") {
        let name_lower = name.to_lowercase();
        // Skip MIDI nodes
        if name_lower.contains("midi") {
            return false;
        }
        // Include known audio nodes
        if name_lower.contains("alsa") 
            || name_lower.contains("speakereq")
            || name_lower.contains("riaa")
            || name_lower.contains("output")
            || name_lower.contains("input")
            || name_lower.contains("sink")
            || name_lower.contains("source")
        {
            return true;
        }
    }
    
    // Default: include nodes without media.class that look like audio
    obj.object_type == "Node" || obj.object_type == "Device"
}

/// Detect filter-chain pairs: 
/// Pattern 1: input (Audio/Sink) + output (Stream/Output/Audio, name=$base.output)
/// Pattern 2: input (Audio/Source/Virtual) + output (Stream/Output/Audio, name=$base.output)
/// Pattern 3: input ($name_input.proc) + output ($name_output.proc)
fn detect_filter_chains(nodes: &[&pwcli::PwObject]) -> Vec<FilterChain> {
    let mut chains = Vec::new();
    let mut used_ids: HashSet<u32> = HashSet::new();
    
    // Build a map of node.name -> (id, media.class)
    let mut name_to_info: HashMap<String, (u32, String)> = HashMap::new();
    for node in nodes {
        if let Some(name) = node.properties.get("node.name") {
            let media_class = node.properties.get("media.class")
                .map(|s| s.as_str())
                .unwrap_or("");
            name_to_info.insert(name.clone(), (node.id, media_class.to_string()));
        }
    }
    
    // Pattern 1 & 2: Find input nodes (Audio/Sink or Audio/Source/Virtual) with matching $name.output
    for node in nodes {
        if let Some(media_class) = node.properties.get("media.class") {
            if media_class == "Audio/Sink" || media_class == "Audio/Source/Virtual" {
                if let Some(name) = node.properties.get("node.name") {
                    let output_name = format!("{}.output", name);
                    if let Some((output_id, output_class)) = name_to_info.get(&output_name) {
                        if output_class == "Stream/Output/Audio" && !used_ids.contains(&node.id) {
                            chains.push(FilterChain {
                                name: name.clone(),
                                input_id: node.id,
                                output_id: *output_id,
                            });
                            used_ids.insert(node.id);
                            used_ids.insert(*output_id);
                        }
                    }
                }
            }
        }
    }
    
    // Pattern 3: Find pairs like foo_input.proc / foo_output.proc
    for node in nodes {
        if let Some(name) = node.properties.get("node.name") {
            if name.ends_with("_input.proc") && !used_ids.contains(&node.id) {
                let base = name.strip_suffix("_input.proc").unwrap();
                let output_name = format!("{}_output.proc", base);
                if let Some((output_id, _)) = name_to_info.get(&output_name) {
                    if !used_ids.contains(output_id) {
                        chains.push(FilterChain {
                            name: base.to_string(),
                            input_id: node.id,
                            output_id: *output_id,
                        });
                        used_ids.insert(node.id);
                        used_ids.insert(*output_id);
                    }
                }
            }
        }
    }
    
    chains
}

/// Check if a port belongs to an audio node
fn is_audio_port(obj: &pwcli::PwObject, audio_node_ids: &HashSet<u32>) -> bool {
    // Check if port's parent node is an audio node
    if let Some(node_id_str) = obj.properties.get("node.id") {
        if let Ok(node_id) = node_id_str.parse::<u32>() {
            return audio_node_ids.contains(&node_id);
        }
    }
    false
}

/// Generate DOT format graph of audio topology
fn generate_dot_graph(objects: &[pwcli::PwObject]) -> String {
    let mut dot = String::new();
    
    dot.push_str("digraph PipeWire {\n");
    dot.push_str("    rankdir=LR;\n");
    dot.push_str("    node [shape=box, style=filled];\n");
    dot.push_str("    \n");
    
    // Collect audio nodes and their IDs
    let mut audio_node_ids: HashSet<u32> = HashSet::new();
    let mut nodes: Vec<&pwcli::PwObject> = Vec::new();
    let mut devices: Vec<&pwcli::PwObject> = Vec::new();
    
    for obj in objects {
        if obj.object_type == "Node" && is_audio_node(obj) {
            audio_node_ids.insert(obj.id);
            nodes.push(obj);
        } else if obj.object_type == "Device" && is_audio_node(obj) {
            devices.push(obj);
        }
    }
    
    // Detect filter-chains (input + output pairs)
    let filter_chains = detect_filter_chains(&nodes);
    
    // Build sets for filter-chain node IDs
    let mut filter_chain_input_ids: HashSet<u32> = HashSet::new();
    let mut filter_chain_output_ids: HashSet<u32> = HashSet::new();
    let mut filter_chain_map: HashMap<u32, &FilterChain> = HashMap::new(); // maps input_id or output_id -> chain
    
    for chain in &filter_chains {
        filter_chain_input_ids.insert(chain.input_id);
        filter_chain_output_ids.insert(chain.output_id);
        filter_chain_map.insert(chain.input_id, chain);
        filter_chain_map.insert(chain.output_id, chain);
    }
    
    // Collect audio ports
    let mut ports: HashMap<u32, &pwcli::PwObject> = HashMap::new();
    let mut port_to_node: HashMap<u32, u32> = HashMap::new();
    
    for obj in objects {
        if obj.object_type == "Port" && is_audio_port(obj, &audio_node_ids) {
            ports.insert(obj.id, obj);
            if let Some(node_id_str) = obj.properties.get("node.id") {
                if let Ok(node_id) = node_id_str.parse::<u32>() {
                    port_to_node.insert(obj.id, node_id);
                }
            }
        }
    }
    
    // Collect links between audio ports
    let mut links: Vec<&pwcli::PwObject> = Vec::new();
    for obj in objects {
        if obj.object_type == "Link" {
            // Check if both ports are audio ports
            let out_port_id = obj.properties.get("link.output.port")
                .and_then(|s| s.parse::<u32>().ok());
            let in_port_id = obj.properties.get("link.input.port")
                .and_then(|s| s.parse::<u32>().ok());
            
            if let (Some(out_id), Some(in_id)) = (out_port_id, in_port_id) {
                if ports.contains_key(&out_id) || ports.contains_key(&in_id) {
                    links.push(obj);
                }
            }
        }
    }
    
    // Add devices subgraph
    if !devices.is_empty() {
        dot.push_str("    subgraph cluster_devices {\n");
        dot.push_str("        label=\"Devices\";\n");
        dot.push_str("        style=dashed;\n");
        dot.push_str("        color=gray;\n");
        for device in &devices {
            let name = device.display_name();
            let escaped_name = name.replace('"', "\\\"");
            dot.push_str(&format!(
                "        dev_{} [label=\"{}\", fillcolor=lightgray];\n",
                device.id, escaped_name
            ));
        }
        dot.push_str("    }\n\n");
    }
    
    // Add filter-chains as combined nodes
    if !filter_chains.is_empty() {
        dot.push_str("    // Filter Chains (combined input+output)\n");
        for chain in &filter_chains {
            let escaped_name = chain.name.replace('"', "\\\"");
            dot.push_str(&format!(
                "    chain_{} [label=\"{}\\nID: {}/{}\", fillcolor=lightyellow, style=\"filled,bold\"];\n",
                chain.input_id, escaped_name, chain.input_id, chain.output_id
            ));
        }
        dot.push('\n');
    }
    
    // Add regular nodes (excluding filter-chain members)
    dot.push_str("    // Audio Nodes\n");
    for node in &nodes {
        // Skip nodes that are part of a filter-chain
        if filter_chain_input_ids.contains(&node.id) || filter_chain_output_ids.contains(&node.id) {
            continue;
        }
        
        let name = node.display_name();
        let escaped_name = name.replace('"', "\\\"");
        
        // Determine color based on media.class
        let color = if let Some(media_class) = node.properties.get("media.class") {
            let class_lower = media_class.to_lowercase();
            if class_lower.contains("sink") || class_lower.contains("playback") {
                "lightblue"
            } else if class_lower.contains("source") || class_lower.contains("capture") {
                "lightgreen"
            } else if class_lower.contains("filter") {
                "lightyellow"
            } else {
                "white"
            }
        } else {
            "white"
        };
        
        dot.push_str(&format!(
            "    node_{} [label=\"{}\\nID: {}\", fillcolor={}];\n",
            node.id, escaped_name, node.id, color
        ));
    }
    dot.push('\n');
    
    // Add links between nodes (aggregate port links to node links)
    // For filter-chains, map input/output node IDs to the chain's input_id
    dot.push_str("    // Links\n");
    let mut node_links: HashSet<(String, String)> = HashSet::new();
    
    // Helper to get the graph node name for a PipeWire node ID
    let get_graph_node = |node_id: u32| -> String {
        if let Some(chain) = filter_chain_map.get(&node_id) {
            format!("chain_{}", chain.input_id)
        } else {
            format!("node_{}", node_id)
        }
    };
    
    for link in &links {
        let out_port_id = link.properties.get("link.output.port")
            .and_then(|s| s.parse::<u32>().ok());
        let in_port_id = link.properties.get("link.input.port")
            .and_then(|s| s.parse::<u32>().ok());
        
        if let (Some(out_id), Some(in_id)) = (out_port_id, in_port_id) {
            if let (Some(&out_node), Some(&in_node)) = (port_to_node.get(&out_id), port_to_node.get(&in_id)) {
                // Only add if both nodes are audio nodes
                if audio_node_ids.contains(&out_node) && audio_node_ids.contains(&in_node) {
                    let from = get_graph_node(out_node);
                    let to = get_graph_node(in_node);
                    // Skip internal filter-chain links (input -> output within same chain)
                    if from != to {
                        node_links.insert((from, to));
                    }
                }
            }
        }
    }
    
    for (from, to) in node_links {
        dot.push_str(&format!("    {} -> {};\n", from, to));
    }
    
    // Add legend
    dot.push_str("\n    // Legend\n");
    dot.push_str("    subgraph cluster_legend {\n");
    dot.push_str("        label=\"Legend\";\n");
    dot.push_str("        style=solid;\n");
    dot.push_str("        legend_sink [label=\"Sink/Playback\", fillcolor=lightblue];\n");
    dot.push_str("        legend_source [label=\"Source/Capture\", fillcolor=lightgreen];\n");
    dot.push_str("        legend_filter [label=\"Filter\", fillcolor=lightyellow];\n");
    dot.push_str("        legend_chain [label=\"Filter-Chain\", fillcolor=lightyellow, style=\"filled,bold\"];\n");
    dot.push_str("    }\n");
    
    dot.push_str("}\n");
    
    dot
}

/// Handler for GET /api/v1/graph - returns DOT format graph
pub async fn get_graph_dot(
    State(_state): State<Arc<AppState>>,
) -> Response {
    // Get all objects
    let objects = match pwcli::list_all() {
        Ok(objs) => objs,
        Err(e) => {
            error!("Failed to list PipeWire objects: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get PipeWire objects").into_response();
        }
    };
    
    let dot = generate_dot_graph(&objects);
    
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/vnd.graphviz")],
        dot,
    ).into_response()
}

/// Handler for GET /api/v1/graph/png - returns PNG image
pub async fn get_graph_png(
    State(_state): State<Arc<AppState>>,
) -> Response {
    // Check if graphviz (dot) is available
    let dot_check = Command::new("which")
        .arg("dot")
        .output();
    
    match dot_check {
        Ok(output) if output.status.success() => {
            // dot is available
        }
        _ => {
            error!("Graphviz 'dot' command not found. Install graphviz to enable PNG graph generation.");
            return (StatusCode::NOT_FOUND, "Graphviz not found").into_response();
        }
    }
    
    // Get all objects
    let objects = match pwcli::list_all() {
        Ok(objs) => objs,
        Err(e) => {
            error!("Failed to list PipeWire objects: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get PipeWire objects").into_response();
        }
    };
    
    let dot = generate_dot_graph(&objects);
    
    // Run dot to generate PNG
    let mut child = match Command::new("dot")
        .arg("-Tpng")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(e) => {
            error!("Failed to spawn dot process: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to run graphviz").into_response();
        }
    };
    
    // Write DOT to stdin
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        if let Err(e) = stdin.write_all(dot.as_bytes()) {
            error!("Failed to write to dot stdin: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to generate graph").into_response();
        }
    }
    
    // Get output
    let output = match child.wait_with_output() {
        Ok(output) => output,
        Err(e) => {
            error!("Failed to wait for dot process: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to generate graph").into_response();
        }
    };
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("dot command failed: {}", stderr);
        return (StatusCode::INTERNAL_SERVER_ERROR, "Graphviz error").into_response();
    }
    
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "image/png")],
        output.stdout,
    ).into_response()
}

/// Create router for graph endpoints
pub fn create_graph_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v1/graph", get(get_graph_dot))
        .route("/api/v1/graph/png", get(get_graph_png))
}
