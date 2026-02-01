//! Parser for pw-cli command output
//!
//! This module provides a simpler interface to PipeWire by parsing
//! the output of pw-cli commands instead of using the native API.

use std::collections::HashMap;
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use regex::Regex;
use serde::{Deserialize, Serialize};

// Simple cache for node name <-> ID lookups to avoid repeated pw-cli calls
static NODE_CACHE: OnceLock<Mutex<HashMap<String, u32>>> = OnceLock::new();

/// Classification of PipeWire node types based on media.class
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeTypeClassification {
    /// Audio node (sink, source, filter, stream)
    Audio,
    /// MIDI node
    Midi,
    /// Video node
    Video,
    /// Link object
    Link,
    /// Port object
    Port,
    /// Client connection
    Client,
    /// Other known type (modules, factories, etc.)
    Other,
    /// Unknown - media.class not recognized or missing, needs heuristics
    Unknown,
}

/// Classify a media.class string to determine the node type
/// 
/// Returns the detected node type based on media.class patterns:
/// - `Audio`: Contains "audio" or "stream"
/// - `Midi`: Contains "midi"
/// - `Video`: Contains "video"
/// - `Link`: Is exactly "link" or contains "link/"
/// - `Other`: Has a media.class but doesn't match known patterns
/// - `Unknown`: No media.class provided, caller should use heuristics
/// 
/// # Examples
/// ```
/// use pw_api::pwcli::{classify_media_class, NodeTypeClassification};
/// 
/// assert_eq!(classify_media_class(Some("Audio/Sink")), NodeTypeClassification::Audio);
/// assert_eq!(classify_media_class(Some("Stream/Output/Audio")), NodeTypeClassification::Audio);
/// assert_eq!(classify_media_class(Some("Midi/Bridge")), NodeTypeClassification::Midi);
/// assert_eq!(classify_media_class(Some("Video/Source")), NodeTypeClassification::Video);
/// assert_eq!(classify_media_class(None), NodeTypeClassification::Unknown);
/// ```
pub fn classify_media_class(media_class: Option<&str>) -> NodeTypeClassification {
    match media_class {
        Some(class) => {
            let class_lower = class.to_lowercase();
            
            // Check for audio (includes Stream/Output/Audio patterns)
            if class_lower.contains("audio") || class_lower.contains("stream") {
                return NodeTypeClassification::Audio;
            }
            
            // Check for MIDI
            if class_lower.contains("midi") {
                return NodeTypeClassification::Midi;
            }
            
            // Check for video
            if class_lower.contains("video") {
                return NodeTypeClassification::Video;
            }
            
            // Check for link
            if class_lower == "link" || class_lower.starts_with("link/") {
                return NodeTypeClassification::Link;
            }
            
            // Has media.class but doesn't match known patterns
            NodeTypeClassification::Other
        }
        // No media.class at all - needs heuristics
        None => NodeTypeClassification::Unknown,
    }
}

/// Initialize or refresh the node name cache
fn refresh_node_cache() -> Result<(), String> {
    let nodes = list_nodes()?;
    let mut cache = HashMap::new();
    
    for node in nodes {
        if let Some(name) = node.properties.get("node.name") {
            cache.insert(name.clone(), node.id);
        }
    }
    
    tracing::debug!("refresh_node_cache: loaded {} nodes into cache: {:?}", 
        cache.len(), cache.keys().collect::<Vec<_>>());
    
    let cache_mutex = NODE_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    *cache_mutex.lock().unwrap() = cache;
    Ok(())
}

/// A PipeWire object as returned by pw-cli ls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PwObject {
    pub id: u32,
    pub object_type: String,
    pub properties: HashMap<String, String>,
}

impl PwObject {
    /// Get a property value
    pub fn get(&self, key: &str) -> Option<&str> {
        self.properties.get(key).map(|s| s.as_str())
    }
    
    /// Get the node.name or device.name property
    pub fn name(&self) -> Option<&str> {
        self.get("node.name")
            .or_else(|| self.get("device.name"))
            .or_else(|| self.get("port.name"))
            .or_else(|| self.get("client.name"))
            .or_else(|| self.get("module.name"))
            .or_else(|| self.get("factory.name"))
            .or_else(|| self.get("link.name"))
    }
    
    /// Get a display name for any object type
    /// For links, constructs a name from port/node IDs
    pub fn display_name(&self) -> String {
        // First try the normal name properties
        if let Some(name) = self.name() {
            return name.to_string();
        }
        
        // For links, construct a name from the port/node IDs
        if self.object_type == "Link" {
            let out_node = self.get("link.output.node").unwrap_or("?");
            let out_port = self.get("link.output.port").unwrap_or("?");
            let in_node = self.get("link.input.node").unwrap_or("?");
            let in_port = self.get("link.input.port").unwrap_or("?");
            return format!("{}:{} -> {}:{}", out_node, out_port, in_node, in_port);
        }
        
        // Fall back to object.path or object.serial
        self.get("object.path")
            .or_else(|| self.get("object.serial"))
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("object-{}", self.id))
    }
    
    /// Get the node.description or device.description property
    pub fn description(&self) -> Option<&str> {
        self.get("node.description")
            .or_else(|| self.get("device.description"))
            .or_else(|| self.get("port.alias"))
    }
    
    /// Get the media.class property
    pub fn media_class(&self) -> Option<&str> {
        self.get("media.class")
    }
    
    /// Check if this is a specific type
    pub fn is_type(&self, type_name: &str) -> bool {
        self.object_type.contains(type_name)
    }
}

/// Object type constants matching PipeWire types
pub const TYPE_CORE: &str = "Core";
pub const TYPE_MODULE: &str = "Module";
pub const TYPE_NODE: &str = "Node";
pub const TYPE_DEVICE: &str = "Device";
pub const TYPE_PORT: &str = "Port";
pub const TYPE_FACTORY: &str = "Factory";
pub const TYPE_CLIENT: &str = "Client";
pub const TYPE_LINK: &str = "Link";
pub const TYPE_METADATA: &str = "Metadata";

/// Run pw-cli ls and parse the output
/// 
/// If `filter` is provided, only objects of that type are returned.
/// Valid filters: Node, Device, Port, Module, Factory, Client, Link, etc.
pub fn list_objects(filter: Option<&str>) -> Result<Vec<PwObject>, String> {
    let mut cmd = Command::new("pw-cli");
    cmd.arg("ls");
    
    if let Some(f) = filter {
        cmd.arg(f);
    }
    
    let output = cmd
        .output()
        .map_err(|e| format!("Failed to run pw-cli ls: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("pw-cli ls failed: {}", stderr));
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_pwcli_ls(&stdout)
}

/// List all objects
pub fn list_all() -> Result<Vec<PwObject>, String> {
    list_objects(None)
}

/// List nodes only
pub fn list_nodes() -> Result<Vec<PwObject>, String> {
    list_objects(Some(TYPE_NODE))
}

/// List devices only
pub fn list_devices() -> Result<Vec<PwObject>, String> {
    list_objects(Some(TYPE_DEVICE))
}

/// List ports only
pub fn list_ports() -> Result<Vec<PwObject>, String> {
    list_objects(Some(TYPE_PORT))
}

/// List modules only
pub fn list_modules() -> Result<Vec<PwObject>, String> {
    list_objects(Some(TYPE_MODULE))
}

/// List factories only
pub fn list_factories() -> Result<Vec<PwObject>, String> {
    list_objects(Some(TYPE_FACTORY))
}

/// List clients only
pub fn list_clients() -> Result<Vec<PwObject>, String> {
    list_objects(Some(TYPE_CLIENT))
}

/// List links only
pub fn list_links() -> Result<Vec<PwObject>, String> {
    list_objects(Some(TYPE_LINK))
}

/// Get a specific object by ID
pub fn get_object(id: u32) -> Result<Option<PwObject>, String> {
    let objects = list_all()?;
    Ok(objects.into_iter().find(|o| o.id == id))
}

/// Find a node by name using cache (refreshes cache on first call or if not found)
pub fn find_node_by_name(name: &str) -> Result<Option<PwObject>, String> {
    // Initialize cache on first use
    if NODE_CACHE.get().is_none() {
        refresh_node_cache()?;
    }
    
    // Try to find in cache
    let cache = NODE_CACHE.get().unwrap().lock().unwrap();
    if let Some(&id) = cache.get(name) {
        drop(cache); // Release lock before calling get_object
        return get_object(id);
    }
    
    // Not in cache - refresh and try again
    drop(cache);
    refresh_node_cache()?;
    
    let cache = NODE_CACHE.get().unwrap().lock().unwrap();
    if let Some(&id) = cache.get(name) {
        drop(cache);
        return get_object(id);
    }
    
    Ok(None)
}

/// Find a node name by ID using cache
pub fn find_name_by_id(id: u32) -> Result<Option<String>, String> {
    // Initialize cache on first use
    if NODE_CACHE.get().is_none() {
        refresh_node_cache()?;
    }
    
    // Search cache for this ID
    let cache = NODE_CACHE.get().unwrap().lock().unwrap();
    let name = cache.iter()
        .find(|(_, &node_id)| node_id == id)
        .map(|(name, _)| name.clone());
    
    Ok(name)
}

/// Find a node by regex pattern (e.g., "speakereq[0-9]x[0-9]" finds "speakereq2x2", "speakereq4x4", etc.)
/// The pattern must match the entire node name (anchored with ^ and $).
pub fn find_node_by_match(pattern: &str) -> Result<Option<PwObject>, String> {
    tracing::debug!("find_node_by_match: looking for pattern '{}'", pattern);
    
    // Compile the regex with anchors to match the entire name
    let anchored_pattern = format!("^{}$", pattern);
    let re = Regex::new(&anchored_pattern)
        .map_err(|e| format!("Invalid regex pattern '{}': {}", pattern, e))?;
    
    // Initialize cache on first use
    if NODE_CACHE.get().is_none() {
        tracing::debug!("find_node_by_match: cache not initialized, refreshing");
        refresh_node_cache()?;
    }
    
    // Search cache for a name matching the pattern
    let cache = NODE_CACHE.get().unwrap().lock().unwrap();
    tracing::debug!("find_node_by_match: searching cache with {} entries", cache.len());
    let found = cache.iter()
        .find(|(name, _)| re.is_match(name))
        .map(|(name, &id)| {
            tracing::debug!("find_node_by_match: found '{}' with id {} in cache", name, id);
            id
        });
    
    if let Some(id) = found {
        drop(cache);
        let obj = get_object(id)?;
        tracing::debug!("find_node_by_match: get_object({}) returned {:?}", id, obj.as_ref().map(|o| o.name()));
        return Ok(obj);
    }
    
    // Not in cache - refresh and try again
    tracing::debug!("find_node_by_match: not found in cache, refreshing");
    drop(cache);
    refresh_node_cache()?;
    
    let cache = NODE_CACHE.get().unwrap().lock().unwrap();
    let found = cache.iter()
        .find(|(name, _)| re.is_match(name))
        .map(|(name, &id)| {
            tracing::debug!("find_node_by_match: found '{}' with id {} after refresh", name, id);
            id
        });
    
    if let Some(id) = found {
        drop(cache);
        let obj = get_object(id)?;
        tracing::debug!("find_node_by_match: get_object({}) returned {:?}", id, obj.as_ref().map(|o| o.name()));
        return Ok(obj);
    }
    
    tracing::warn!("find_node_by_match: no node found matching pattern '{}'", pattern);
    Ok(None)
}

/// Parse pw-cli ls output into objects
/// 
/// Format:
/// ```text
///         id 38, type PipeWire:Interface:Node/3
///                 object.serial = "38"
///                 factory.id = "18"
///                 node.name = "effect_input.proc"
/// ```
fn parse_pwcli_ls(output: &str) -> Result<Vec<PwObject>, String> {
    let mut objects = Vec::new();
    
    // Regex for object header: "id N, type PipeWire:Interface:Type/Version"
    let header_re = Regex::new(r"^\s*id\s+(\d+),\s+type\s+PipeWire:Interface:(\w+)/\d+")
        .map_err(|e| format!("Invalid header regex: {}", e))?;
    
    // Regex for property: "key = "value"" or "key = value"
    let prop_re = Regex::new(r#"^\s+(\S+)\s+=\s+"?([^"]*)"?\s*$"#)
        .map_err(|e| format!("Invalid property regex: {}", e))?;
    
    let mut current_object: Option<PwObject> = None;
    
    for line in output.lines() {
        if let Some(caps) = header_re.captures(line) {
            // Save previous object if any
            if let Some(obj) = current_object.take() {
                objects.push(obj);
            }
            
            // Start new object
            let id: u32 = caps.get(1).unwrap().as_str().parse().unwrap_or(0);
            let object_type = caps.get(2).unwrap().as_str().to_string();
            
            current_object = Some(PwObject {
                id,
                object_type,
                properties: HashMap::new(),
            });
        } else if let Some(caps) = prop_re.captures(line) {
            // Add property to current object
            if let Some(ref mut obj) = current_object {
                let key = caps.get(1).unwrap().as_str().to_string();
                let value = caps.get(2).unwrap().as_str().to_string();
                obj.properties.insert(key, value);
            }
        }
    }
    
    // Don't forget the last object
    if let Some(obj) = current_object {
        objects.push(obj);
    }
    
    Ok(objects)
}

/// Map PipeWire object type to simple type name
pub fn simplify_type(pw_type: &str) -> &str {
    match pw_type {
        "Node" => "node",
        "Device" => "device",
        "Port" => "port",
        "Module" => "module",
        "Factory" => "factory",
        "Client" => "client",
        "Link" => "link",
        "Core" => "core",
        "Metadata" => "metadata",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_pwcli_ls() {
        let output = r#"
        id 38, type PipeWire:Interface:Node/3
                object.serial = "38"
                factory.id = "18"
                client.id = "35"
                node.description = "EQ + Balance Sink"
                node.name = "effect_input.proc"
                media.class = "Audio/Sink"
        id 66, type PipeWire:Interface:Node/3
                object.serial = "66"
                factory.id = "18"
                node.name = "alsa_output.hdmi"
                node.description = "Built-in Audio Digital Stereo (HDMI)"
                media.class = "Audio/Sink"
        id 67, type PipeWire:Interface:Device/3
                object.serial = "67"
                device.name = "alsa_card.0"
                device.description = "Built-in Audio"
"#;
        
        let objects = parse_pwcli_ls(output).unwrap();
        
        assert_eq!(objects.len(), 3);
        
        // Check first node
        let node1 = &objects[0];
        assert_eq!(node1.id, 38);
        assert_eq!(node1.object_type, "Node");
        assert_eq!(node1.name(), Some("effect_input.proc"));
        assert_eq!(node1.description(), Some("EQ + Balance Sink"));
        assert_eq!(node1.media_class(), Some("Audio/Sink"));
        
        // Check second node
        let node2 = &objects[1];
        assert_eq!(node2.id, 66);
        assert_eq!(node2.name(), Some("alsa_output.hdmi"));
        
        // Check device
        let device = &objects[2];
        assert_eq!(device.id, 67);
        assert_eq!(device.object_type, "Device");
        assert_eq!(device.name(), Some("alsa_card.0"));
    }
    
    #[test]
    fn test_simplify_type() {
        assert_eq!(simplify_type("Node"), "node");
        assert_eq!(simplify_type("Device"), "device");
        assert_eq!(simplify_type("Unknown"), "unknown");
    }
}
