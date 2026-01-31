use anyhow::{anyhow, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;
use pipewire as pw;

/// Link operation type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LinkType {
    Link,
    Unlink,
}

/// Node identifier - can use node.name, node.nick, or object.path with wildcard support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeIdentifier {
    #[serde(rename = "node.name")]
    pub node_name: Option<String>,
    #[serde(rename = "node.nick")]
    pub node_nick: Option<String>,
    #[serde(rename = "object.path")]
    pub object_path: Option<String>,
}

impl NodeIdentifier {
    /// Check if a node matches this identifier
    pub fn matches(&self, props: &pw::spa::utils::dict::DictRef) -> bool {
        if let Some(ref pattern) = self.node_name {
            if let Some(name) = props.get("node.name") {
                if regex_match(pattern, name) {
                    return true;
                }
            }
        }
        
        if let Some(ref pattern) = self.node_nick {
            if let Some(nick) = props.get("node.nick") {
                if regex_match(pattern, nick) {
                    return true;
                }
            }
        }
        
        if let Some(ref pattern) = self.object_path {
            if let Some(path) = props.get("object.path") {
                if regex_match(pattern, path) {
                    return true;
                }
            }
        }
        
        false
    }
}

/// A link rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkRule {
    /// Name of the link rule (used for the created link objects)
    pub name: String,
    pub source: NodeIdentifier,
    pub destination: NodeIdentifier,
    #[serde(rename = "type")]
    pub link_type: LinkType,
    /// Whether to apply this rule at startup (default: true)
    #[serde(default = "default_link_at_startup")]
    pub link_at_startup: bool,
    /// How often to check and relink in seconds. 0 = link once only (default: 0)
    #[serde(default)]
    pub relink_every: u64,
}

fn default_link_at_startup() -> bool {
    true
}

/// Information about a found node
#[derive(Debug, Clone)]
struct NodeMatch {
    id: u32,
    name: String,
}

/// Match a string against a regex pattern
fn regex_match(pattern: &str, text: &str) -> bool {
    if let Ok(re) = Regex::new(pattern) {
        re.is_match(text)
    } else {
        false
    }
}

/// Information about a found node with its properties
#[derive(Debug, Clone)]
struct NodeWithProps {
    id: u32,
    node_name: Option<String>,
    node_nick: Option<String>,
    object_path: Option<String>,
}

/// Check if a node matches an identifier
fn matches_identifier(node: &NodeWithProps, identifier: &NodeIdentifier) -> bool {
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


/// Destroy a link
pub fn destroy_link(
    registry: &pw::registry::RegistryRc,
    mainloop: &pw::main_loop::MainLoopRc,
    link_id: u32,
) -> Result<()> {
    // Destroy the link object
    registry.destroy_global(link_id);
    
    // Give it a moment to process
    let timeout_mainloop = mainloop.clone();
    let _timer = mainloop.loop_().add_timer(move |_| {
        timeout_mainloop.quit();
    });
    _timer.update_timer(Some(std::time::Duration::from_millis(100)), None);
    mainloop.run();
    
    Ok(())
}

/// Apply a link rule
pub fn apply_rule(
    registry: &pw::registry::RegistryRc,
    mainloop: &pw::main_loop::MainLoopRc,
    rule: &LinkRule,
) -> Result<Vec<String>> {
    let mut results = Vec::new();
    
    // Collect ALL nodes and ports in a single pass
    let all_nodes: Rc<RefCell<Vec<NodeWithProps>>> = Rc::new(RefCell::new(Vec::new()));
    let all_nodes_clone = all_nodes.clone();
    
    let all_ports: Rc<RefCell<Vec<(u32, u32, String, bool)>>> = Rc::new(RefCell::new(Vec::new()));
    let all_ports_clone = all_ports.clone();
    
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
                        all_nodes_clone.borrow_mut().push(NodeWithProps {
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
                                
                                all_ports_clone.borrow_mut().push((
                                    global.id,
                                    node_id,
                                    port_name,
                                    is_output,
                                ));
                            }
                        }
                    }
                }
            }
        })
        .register();
    
    mainloop.run();
    
    // Now filter for source nodes
    let mut sources = Vec::new();
    for node in all_nodes.borrow().iter() {
        if matches_identifier(node, &rule.source) {
            let name = node.node_name.as_ref()
                .or(node.node_nick.as_ref())
                .or(node.object_path.as_ref())
                .map(|s| s.as_str())
                .unwrap_or("unknown");
            sources.push(NodeMatch {
                id: node.id,
                name: name.to_string(),
            });
        }
    }
    
    if sources.is_empty() {
        return Err(anyhow!("No source nodes found matching criteria"));
    }
    
    // Filter for destination nodes
    let mut destinations = Vec::new();
    for node in all_nodes.borrow().iter() {
        if matches_identifier(node, &rule.destination) {
            let name = node.node_name.as_ref()
                .or(node.node_nick.as_ref())
                .or(node.object_path.as_ref())
                .map(|s| s.as_str())
                .unwrap_or("unknown");
            destinations.push(NodeMatch {
                id: node.id,
                name: name.to_string(),
            });
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
                    // Find ports for these nodes
                    let mut source_outputs = Vec::new();
                    let mut dest_inputs = Vec::new();
                    
                    for (port_id, node_id, port_name, is_output) in all_ports.borrow().iter() {
                        if *node_id == source.id && *is_output {
                            source_outputs.push((*port_id, port_name.clone()));
                        } else if *node_id == dest.id && !*is_output {
                            dest_inputs.push((*port_id, port_name.clone()));
                        }
                    }
                    
                    // Check port counts match
                    if source_outputs.len() != dest_inputs.len() {
                        let msg = format!(
                            "Port count mismatch for {} -> {}: {} output ports vs {} input ports",
                            source.name, dest.name, source_outputs.len(), dest_inputs.len()
                        );
                        results.push(msg);
                        continue;
                    }
                    
                    if source_outputs.is_empty() {
                        let msg = format!("No ports found to link {} -> {}", source.name, dest.name);
                        results.push(msg);
                        continue;
                    }
                    
                    // Sort ports by ID to ensure consistent ordering
                    source_outputs.sort_by_key(|(id, _)| *id);
                    dest_inputs.sort_by_key(|(id, _)| *id);
                    
                    // List what would be linked
                    for ((src_port_id, src_port_name), (dst_port_id, dst_port_name)) in 
                        source_outputs.iter().zip(dest_inputs.iter()) {
                        let msg = format!(
                            "Would link port {} ({}) to port {} ({})",
                            src_port_id, src_port_name, dst_port_id, dst_port_name
                        );
                        results.push(msg);
                    }
                }
            }
        }
        LinkType::Unlink => {
            // For unlink, we need to find existing links between these nodes
            // This would require querying existing links and matching them
            // Simplified implementation for now
            results.push("Unlink operation not yet fully implemented".to_string());
        }
    }
    
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_regex_match() {
        // Test basic regex patterns
        assert!(regex_match("^test.*", "test123"));
        assert!(regex_match(".*test$", "mytest"));
        assert!(regex_match(".*test.*", "myteststring"));
        assert!(regex_match("^test$", "test"));
        assert!(!regex_match("^test$", "test123"));
        assert!(regex_match("^node\\.", "node.input"));
        
        // Test single character patterns
        assert!(regex_match("^test.$", "test1"));
        assert!(regex_match("^test.$", "testa"));
        assert!(!regex_match("^test.$", "test12"));
        assert!(regex_match("^speakereq.x.\\.output$", "speakereq2x2.output"));
        assert!(regex_match("^speakereq.x.\\.output$", "speakereq4x4.output"));
        
        // Test complex patterns
        assert!(regex_match("alsa.*sndrpihifiberry.*playback", "alsa:acp:sndrpihifiberry:1:playback"));
        assert!(regex_match("alsa:.*:sndrpihifiberry:.*:playback", "alsa:acp:sndrpihifiberry:1:playback"));
        assert!(regex_match("^test..*", "test1234"));
    }
}
