//! Link manager using pw-cli and pw-link commands
//!
//! This module provides link rule management using command-line tools
//! instead of the native PipeWire API for simplicity and reliability.

use crate::linker::{LinkRule, LinkType, NodeIdentifier};
use crate::util::regex_match;
use crate::pwcli::{self, PwObject};
use crate::pwlink;

/// Result of applying a link rule
#[derive(Debug, Clone)]
pub struct LinkRuleResult {
    pub success: bool,
    pub message: String,
}

/// Information about a node from the cache
#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub id: u32,
    pub node_name: Option<String>,
    pub node_nick: Option<String>,
    pub object_path: Option<String>,
}

impl NodeInfo {
    /// Create from a PwObject
    pub fn from_pw_object(obj: &PwObject) -> Self {
        NodeInfo {
            id: obj.id,
            node_name: obj.get("node.name").map(|s| s.to_string()),
            node_nick: obj.get("node.nick").map(|s| s.to_string()),
            object_path: obj.get("object.path").map(|s| s.to_string()),
        }
    }
    
    /// Get a display name for this node
    pub fn display_name(&self) -> String {
        self.node_name.as_ref()
            .or(self.node_nick.as_ref())
            .or(self.object_path.as_ref())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("node-{}", self.id))
    }
}

/// Information about a port from the cache
#[derive(Debug, Clone)]
pub struct PortInfo {
    pub id: u32,
    pub node_id: u32,
    pub name: String,
    pub full_name: String,  // "node_name:port_name" format for pw-link
    pub direction: PortDirection,
    pub channel: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortDirection {
    Input,
    Output,
}

impl PortInfo {
    /// Create from a PwObject, with node lookup for full name
    pub fn from_pw_object(obj: &PwObject, nodes: &[PwObject]) -> Option<Self> {
        let node_id: u32 = obj.get("node.id")?.parse().ok()?;
        let port_name = obj.get("port.name")?.to_string();
        
        // Find the parent node to get its name
        let node = nodes.iter().find(|n| n.id == node_id)?;
        let node_name = node.get("node.name")?;
        
        let direction = match obj.get("port.direction")? {
            "in" => PortDirection::Input,
            "out" => PortDirection::Output,
            _ => return None,
        };
        
        Some(PortInfo {
            id: obj.id,
            node_id,
            name: port_name.clone(),
            full_name: format!("{}:{}", node_name, port_name),
            direction,
            channel: obj.get("audio.channel").map(|s| s.to_string()),
        })
    }
}

/// Information about an existing link
#[derive(Debug, Clone)]
pub struct LinkInfo {
    pub id: u32,
    pub output_port_id: u32,
    pub input_port_id: u32,
    pub output_port_name: String,
    pub input_port_name: String,
}

impl LinkInfo {
    /// Create from a pwlink::PwLink
    pub fn from_pw_link(link: &pwlink::PwLink) -> Self {
        LinkInfo {
            id: link.id,
            output_port_id: link.output_port_id,
            input_port_id: link.input_port_id,
            output_port_name: link.output_port_name.clone(),
            input_port_name: link.input_port_name.clone(),
        }
    }
}

/// Check if a node matches an identifier
fn matches_identifier(node: &NodeInfo, identifier: &NodeIdentifier) -> bool {
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

/// Load all data needed for link management from the cache or fresh
pub struct LinkData {
    pub nodes: Vec<NodeInfo>,
    pub ports: Vec<PortInfo>,
    pub links: Vec<LinkInfo>,
}

impl LinkData {
    /// Load fresh data from PipeWire
    pub fn load() -> Result<Self, String> {
        // Load all objects
        let all_objects = pwcli::list_all()?;
        
        // Extract nodes
        let nodes: Vec<NodeInfo> = all_objects.iter()
            .filter(|o| o.object_type == "Node")
            .map(NodeInfo::from_pw_object)
            .collect();
        
        // Extract ports (needs node info for full names)
        let node_objects: Vec<&PwObject> = all_objects.iter()
            .filter(|o| o.object_type == "Node")
            .collect();
        
        let ports: Vec<PortInfo> = all_objects.iter()
            .filter(|o| o.object_type == "Port")
            .filter_map(|o| {
                let node_id: u32 = o.get("node.id")?.parse().ok()?;
                let port_name = o.get("port.name")?.to_string();
                
                // Find the parent node
                let node = node_objects.iter().find(|n| n.id == node_id)?;
                let node_name = node.get("node.name")?;
                
                let direction = match o.get("port.direction")? {
                    "in" => PortDirection::Input,
                    "out" => PortDirection::Output,
                    _ => return None,
                };
                
                // Skip monitor ports (they're output copies)
                if o.get("port.monitor").map(|v| v == "true").unwrap_or(false) {
                    return None;
                }
                
                Some(PortInfo {
                    id: o.id,
                    node_id,
                    name: port_name.clone(),
                    full_name: format!("{}:{}", node_name, port_name),
                    direction,
                    channel: o.get("audio.channel").map(|s| s.to_string()),
                })
            })
            .collect();
        
        // Load existing links
        let pw_links = pwlink::list_links()?;
        let links: Vec<LinkInfo> = pw_links.iter()
            .map(LinkInfo::from_pw_link)
            .collect();
        
        Ok(LinkData { nodes, ports, links })
    }
    
    /// Find nodes matching an identifier
    pub fn find_matching_nodes(&self, identifier: &NodeIdentifier) -> Vec<&NodeInfo> {
        self.nodes.iter()
            .filter(|n| matches_identifier(n, identifier))
            .collect()
    }
    
    /// Get output ports for a node
    pub fn get_output_ports(&self, node_id: u32) -> Vec<&PortInfo> {
        self.ports.iter()
            .filter(|p| p.node_id == node_id && p.direction == PortDirection::Output)
            .collect()
    }
    
    /// Get input ports for a node
    pub fn get_input_ports(&self, node_id: u32) -> Vec<&PortInfo> {
        self.ports.iter()
            .filter(|p| p.node_id == node_id && p.direction == PortDirection::Input)
            .collect()
    }
    
    /// Check if a link exists between two ports (by full name)
    pub fn link_exists(&self, output_name: &str, input_name: &str) -> bool {
        self.links.iter().any(|l| 
            l.output_port_name == output_name && l.input_port_name == input_name
        )
    }
    
    /// Check if a link exists between two ports (by ID)
    pub fn link_exists_by_id(&self, output_id: u32, input_id: u32) -> bool {
        self.links.iter().any(|l| 
            l.output_port_id == output_id && l.input_port_id == input_id
        )
    }
    
    /// Find link ID for a connection
    pub fn find_link_id(&self, output_name: &str, input_name: &str) -> Option<u32> {
        self.links.iter()
            .find(|l| l.output_port_name == output_name && l.input_port_name == input_name)
            .map(|l| l.id)
    }
}

/// Apply a link rule and return results
pub fn apply_link_rule(rule: &LinkRule) -> Result<Vec<LinkRuleResult>, String> {
    let mut results = Vec::new();
    
    // Load current state
    let data = LinkData::load()?;
    
    // Find source nodes
    let sources = data.find_matching_nodes(&rule.source);
    if sources.is_empty() {
        return Err("No source nodes found matching criteria".to_string());
    }
    
    // Find destination nodes
    let destinations = data.find_matching_nodes(&rule.destination);
    if destinations.is_empty() {
        return Err("No destination nodes found matching criteria".to_string());
    }
    
    match rule.link_type {
        LinkType::Link => {
            for source in &sources {
                for dest in &destinations {
                    let source_outputs = data.get_output_ports(source.id);
                    let dest_inputs = data.get_input_ports(dest.id);
                    
                    // Check port counts match
                    if source_outputs.len() != dest_inputs.len() {
                        results.push(LinkRuleResult {
                            success: false,
                            message: format!(
                                "Port count mismatch for {} -> {}: {} output ports vs {} input ports",
                                source.display_name(), dest.display_name(),
                                source_outputs.len(), dest_inputs.len()
                            ),
                        });
                        continue;
                    }
                    
                    if source_outputs.is_empty() {
                        results.push(LinkRuleResult {
                            success: false,
                            message: format!(
                                "No ports found to link {} -> {}",
                                source.display_name(), dest.display_name()
                            ),
                        });
                        continue;
                    }
                    
                    // Sort ports by ID for consistent ordering
                    let mut source_outputs: Vec<_> = source_outputs;
                    let mut dest_inputs: Vec<_> = dest_inputs;
                    source_outputs.sort_by_key(|p| p.id);
                    dest_inputs.sort_by_key(|p| p.id);
                    
                    // Create links for each port pair
                    for (src_port, dst_port) in source_outputs.iter().zip(dest_inputs.iter()) {
                        // Check if link already exists
                        if data.link_exists(&src_port.full_name, &dst_port.full_name) {
                            results.push(LinkRuleResult {
                                success: true,
                                message: format!(
                                    "Link already exists: {} -> {}",
                                    src_port.full_name, dst_port.full_name
                                ),
                            });
                            continue;
                        }
                        
                        // Create the link using pw-link
                        match pwlink::create_link(&src_port.full_name, &dst_port.full_name) {
                            Ok(()) => {
                                results.push(LinkRuleResult {
                                    success: true,
                                    message: format!(
                                        "Created link: {} -> {}",
                                        src_port.full_name, dst_port.full_name
                                    ),
                                });
                            }
                            Err(e) => {
                                results.push(LinkRuleResult {
                                    success: false,
                                    message: format!(
                                        "Failed to create link {} -> {}: {}",
                                        src_port.full_name, dst_port.full_name, e
                                    ),
                                });
                            }
                        }
                    }
                }
            }
        }
        LinkType::Unlink => {
            for source in &sources {
                for dest in &destinations {
                    let source_outputs = data.get_output_ports(source.id);
                    let dest_inputs = data.get_input_ports(dest.id);
                    
                    // Sort ports for consistent ordering
                    let mut source_outputs: Vec<_> = source_outputs;
                    let mut dest_inputs: Vec<_> = dest_inputs;
                    source_outputs.sort_by_key(|p| p.id);
                    dest_inputs.sort_by_key(|p| p.id);
                    
                    // Remove links for each port pair
                    for (src_port, dst_port) in source_outputs.iter().zip(dest_inputs.iter()) {
                        // Find the link
                        if let Some(link_id) = data.find_link_id(&src_port.full_name, &dst_port.full_name) {
                            match pwlink::remove_link(link_id) {
                                Ok(()) => {
                                    results.push(LinkRuleResult {
                                        success: true,
                                        message: format!(
                                            "Removed link: {} -> {}",
                                            src_port.full_name, dst_port.full_name
                                        ),
                                    });
                                }
                                Err(e) => {
                                    results.push(LinkRuleResult {
                                        success: false,
                                        message: format!(
                                            "Failed to remove link {} -> {}: {}",
                                            src_port.full_name, dst_port.full_name, e
                                        ),
                                    });
                                }
                            }
                        } else {
                            results.push(LinkRuleResult {
                                success: true,
                                message: format!(
                                    "Link does not exist: {} -> {}",
                                    src_port.full_name, dst_port.full_name
                                ),
                            });
                        }
                    }
                }
            }
        }
    }
    
    Ok(results)
}

/// Create a link between two ports by name
pub fn create_link(output: &str, input: &str) -> Result<(), String> {
    pwlink::create_link(output, input)
}

/// Remove a link by ID
pub fn remove_link(link_id: u32) -> Result<(), String> {
    pwlink::remove_link(link_id)
}

/// Remove a link between two ports by name
pub fn remove_link_by_name(output: &str, input: &str) -> Result<(), String> {
    pwlink::remove_link_by_name(output, input)
}

/// List all current links
pub fn list_links() -> Result<Vec<LinkInfo>, String> {
    let pw_links = pwlink::list_links()?;
    Ok(pw_links.iter().map(LinkInfo::from_pw_link).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_matches_identifier() {
        let node = NodeInfo {
            id: 1,
            node_name: Some("effect_output.proc".to_string()),
            node_nick: None,
            object_path: Some("/path/to/node".to_string()),
        };
        
        // Match by node.name
        let id1 = NodeIdentifier {
            node_name: Some("^effect_output\\.proc$".to_string()),
            node_nick: None,
            object_path: None,
        };
        assert!(matches_identifier(&node, &id1));
        
        // Match by object.path
        let id2 = NodeIdentifier {
            node_name: None,
            node_nick: None,
            object_path: Some("/path/.*".to_string()),
        };
        assert!(matches_identifier(&node, &id2));
        
        // No match
        let id3 = NodeIdentifier {
            node_name: Some("^other_node$".to_string()),
            node_nick: None,
            object_path: None,
        };
        assert!(!matches_identifier(&node, &id3));
    }
}
