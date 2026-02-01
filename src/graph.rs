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

/// Check if a node is an audio node (not MIDI, video, or link)
/// Uses classify_media_class for primary classification, then applies
/// additional heuristics for "Unknown" cases.
fn is_audio_node(obj: &pwcli::PwObject) -> bool {
    // First, check media.class using the central classification function
    let classification = pwcli::classify_media_class(obj.media_class());
    
    match classification {
        pwcli::NodeTypeClassification::Audio => return true,
        pwcli::NodeTypeClassification::Midi => return false,
        pwcli::NodeTypeClassification::Video => return false,
        pwcli::NodeTypeClassification::Link => return false,
        pwcli::NodeTypeClassification::Port => return false,
        pwcli::NodeTypeClassification::Client => return false,
        pwcli::NodeTypeClassification::Driver => return false,
        pwcli::NodeTypeClassification::Other => return false,
        pwcli::NodeTypeClassification::Unknown => {
            // Apply additional heuristics for unknown cases
        }
    }
    
    // Additional heuristics for "Unknown" cases (no media.class)
    // Check node.name for known patterns
    if let Some(name) = obj.properties.get("node.name") {
        let name_lower = name.to_lowercase();
        // Skip MIDI nodes
        if name_lower.contains("midi") {
            return false;
        }
        // Skip driver nodes
        if name_lower.contains("driver") {
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
    dot.push_str("    rankdir=TB;\n");
    dot.push_str("    node [shape=box, style=filled];\n");
    dot.push_str("    newrank=true;\n");
    dot.push_str("    compound=true;\n");
    dot.push_str("    \n");
    
    // Collect audio nodes and their IDs
    let mut audio_node_ids: HashSet<u32> = HashSet::new();
    let mut nodes: Vec<&pwcli::PwObject> = Vec::new();
    let mut devices: Vec<&pwcli::PwObject> = Vec::new();
    let mut all_clients: HashMap<u32, &pwcli::PwObject> = HashMap::new();
    let mut node_to_client: HashMap<u32, u32> = HashMap::new();
    
    // First pass: collect clients
    for obj in objects {
        if obj.object_type == "Client" {
            all_clients.insert(obj.id, obj);
        }
    }
    
    // Second pass: collect nodes and map to clients
    for obj in objects {
        if obj.object_type == "Node" && is_audio_node(obj) {
            audio_node_ids.insert(obj.id);
            nodes.push(obj);
            // Track client.id for this node
            if let Some(client_id_str) = obj.properties.get("client.id") {
                if let Ok(client_id) = client_id_str.parse::<u32>() {
                    node_to_client.insert(obj.id, client_id);
                }
            }
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
    
    // Track sources (inputs) and sinks (outputs) for ranking
    let mut source_nodes: Vec<String> = Vec::new();
    let mut sink_nodes: Vec<String> = Vec::new();
    let mut filter_nodes: Vec<String> = Vec::new();
    
    // Track which clients are connected to audio nodes
    let mut connected_client_ids: HashSet<u32> = HashSet::new();
    for (_, client_id) in &node_to_client {
        connected_client_ids.insert(*client_id);
    }
    
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
    
    // Add clients that are connected to audio nodes (filter out internal pipewire/wireplumber clients)
    let connected_clients: Vec<_> = connected_client_ids.iter()
        .filter_map(|id| all_clients.get(id))
        .filter(|client| !client.is_internal_client())
        .collect();
    
    // Create a set of filtered client IDs for edge drawing
    let filtered_client_ids: HashSet<u32> = connected_clients.iter().map(|c| c.id).collect();
    
    // ========== Audio Graph ==========
    
    // Add audio graph in its own cluster
    dot.push_str("    // Audio Graph\n");
    dot.push_str("    subgraph cluster_graph {\n");
    dot.push_str("        label=\"\";\n");
    dot.push_str("        style=invis;\n");
    dot.push_str("        \n");
    
    // Add clients
    if !connected_clients.is_empty() {
        dot.push_str("        // Clients\n");
        for client in &connected_clients {
            let name = client.display_name();
            let escaped_name = name.replace('"', "\\\"");
            dot.push_str(&format!(
                "        client_{} [label=\"{}\\nClient ID: {}\", fillcolor=lavender, shape=ellipse];\n",
                client.id, escaped_name, client.id
            ));
        }
        dot.push_str("\n");
    }
    
    // 4. Add filter-chains as combined nodes
    if !filter_chains.is_empty() {
        dot.push_str("        // Filter Chains (combined input+output)\n");
        for chain in &filter_chains {
            let escaped_name = chain.name.replace('"', "\\\"");
            let node_name = format!("chain_{}", chain.input_id);
            dot.push_str(&format!(
                "        {} [label=\"{}\\nID: {}/{}\", fillcolor=lightyellow, style=\"filled,bold\"];\n",
                node_name, escaped_name, chain.input_id, chain.output_id
            ));
            filter_nodes.push(node_name);
        }
        dot.push('\n');
    }
    
    // Add regular nodes (excluding filter-chain members)
    dot.push_str("        // Audio Nodes\n");
    for node in &nodes {
        // Skip nodes that are part of a filter-chain
        if filter_chain_input_ids.contains(&node.id) || filter_chain_output_ids.contains(&node.id) {
            continue;
        }
        
        let name = node.display_name();
        let escaped_name = name.replace('"', "\\\"");
        let node_name = format!("node_{}", node.id);
        
        // Determine color and category based on media.class
        let (color, category) = if let Some(media_class) = node.properties.get("media.class") {
            let class_lower = media_class.to_lowercase();
            if class_lower.contains("sink") || class_lower.contains("playback") {
                ("lightblue", "sink")
            } else if class_lower.contains("source") || class_lower.contains("capture") {
                ("lightgreen", "source")
            } else if class_lower.contains("filter") {
                ("lightyellow", "filter")
            } else if class_lower.contains("stream/output") {
                ("paleturquoise", "sink")  // Stream outputs are sinks
            } else if class_lower.contains("stream/input") {
                ("palegreen", "source")  // Stream inputs are sources
            } else {
                ("white", "filter")
            }
        } else {
            ("white", "filter")
        };
        
        // Track for ranking
        match category {
            "source" => source_nodes.push(node_name.clone()),
            "sink" => sink_nodes.push(node_name.clone()),
            _ => filter_nodes.push(node_name.clone()),
        }
        
        dot.push_str(&format!(
            "        {} [label=\"{}\\nID: {}\", fillcolor={}];\n",
            node_name, escaped_name, node.id, color
        ));
    }
    dot.push('\n');
    
    // Add links between nodes (aggregate port links to node links)
    // For filter-chains, map input/output node IDs to the chain's input_id
    dot.push_str("        // Links\n");
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
        dot.push_str(&format!("        {} -> {};\n", from, to));
    }
    
    // Add client-to-node connections (dashed lines)
    dot.push_str("\n        // Client connections\n");
    for node in &nodes {
        if filter_chain_input_ids.contains(&node.id) || filter_chain_output_ids.contains(&node.id) {
            continue;
        }
        if let Some(&client_id) = node_to_client.get(&node.id) {
            if filtered_client_ids.contains(&client_id) {
                dot.push_str(&format!(
                    "        client_{} -> node_{} [style=dashed, color=gray];\n",
                    client_id, node.id
                ));
            }
        }
    }
    // Also add client connections for filter-chains (use input node's client)
    for chain in &filter_chains {
        if let Some(&client_id) = node_to_client.get(&chain.input_id) {
            if filtered_client_ids.contains(&client_id) {
                dot.push_str(&format!(
                    "        client_{} -> chain_{} [style=dashed, color=gray];\n",
                    client_id, chain.input_id
                ));
            }
        }
    }
    
    // Close the graph cluster
    dot.push_str("    }\n\n");
    
    // Rank sinks at bottom
    if !sink_nodes.is_empty() {
        dot.push_str("    // Rank: sinks at bottom\n");
        dot.push_str(&format!("    {{ rank=max; {} }}\n", sink_nodes.join("; ")));
    }
    
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
