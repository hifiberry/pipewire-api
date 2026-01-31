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
                if wildcard_match(pattern, name) {
                    return true;
                }
            }
        }
        
        if let Some(ref pattern) = self.node_nick {
            if let Some(nick) = props.get("node.nick") {
                if wildcard_match(pattern, nick) {
                    return true;
                }
            }
        }
        
        if let Some(ref pattern) = self.object_path {
            if let Some(path) = props.get("object.path") {
                if wildcard_match(pattern, path) {
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
    pub source: NodeIdentifier,
    pub destination: NodeIdentifier,
    #[serde(rename = "type")]
    pub link_type: LinkType,
}

/// Information about a found node
#[derive(Debug, Clone)]
struct NodeMatch {
    id: u32,
    name: String,
}

/// Wildcard pattern matching (supports * for any sequence of characters)
fn wildcard_match(pattern: &str, text: &str) -> bool {
    // Convert wildcard pattern to regex
    let regex_pattern = pattern
        .replace(".", "\\.")
        .replace("*", ".*");
    
    if let Ok(re) = Regex::new(&format!("^{}$", regex_pattern)) {
        re.is_match(text)
    } else {
        false
    }
}

/// Find nodes matching a node identifier
fn find_matching_nodes(
    registry: &pw::registry::RegistryRc,
    mainloop: &pw::main_loop::MainLoopRc,
    identifier: &NodeIdentifier,
    timeout_secs: u64,
) -> Result<Vec<NodeMatch>> {
    let found_nodes: Rc<RefCell<Vec<NodeMatch>>> = Rc::new(RefCell::new(Vec::new()));
    let found_nodes_clone = found_nodes.clone();
    
    // Set up timeout
    let timeout_mainloop = mainloop.clone();
    let _timer = mainloop.loop_().add_timer(move |_| {
        timeout_mainloop.quit();
    });
    _timer.update_timer(Some(std::time::Duration::from_secs(timeout_secs)), None);
    
    let _listener = registry
        .add_listener_local()
        .global({
            let identifier = identifier.clone();
            move |global| {
                if global.type_ == pw::types::ObjectType::Node {
                    if let Some(props) = &global.props {
                        if identifier.matches(props) {
                            let name = props.get("node.name")
                                .or_else(|| props.get("node.nick"))
                                .or_else(|| props.get("object.path"))
                                .unwrap_or("unknown");
                            
                            found_nodes_clone.borrow_mut().push(NodeMatch {
                                id: global.id,
                                name: name.to_string(),
                            });
                        }
                    }
                }
            }
        })
        .register();
    
    mainloop.run();
    
    let result = found_nodes.borrow().clone();
    Ok(result)
}

/// Create a link between two nodes
/// TODO: This needs proper PipeWire Core API integration to create links
/// Currently returns an error indicating this feature needs implementation
pub fn create_link(
    _registry: &pw::registry::RegistryRc,
    _mainloop: &pw::main_loop::MainLoopRc,
    source_id: u32,
    dest_id: u32,
) -> Result<u32> {
    // TODO: Implement actual link creation using PipeWire Core API
    // This requires using the core.create_object method with proper parameters
    // For now, return an error to indicate this needs implementation
    Err(anyhow!(
        "Link creation not yet implemented. Would create link from node {} to node {}",
        source_id,
        dest_id
    ))
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
    
    // Find matching source nodes
    let sources = find_matching_nodes(registry, mainloop, &rule.source, 2)?;
    if sources.is_empty() {
        return Err(anyhow!("No source nodes found matching criteria"));
    }
    
    // Find matching destination nodes
    let destinations = find_matching_nodes(registry, mainloop, &rule.destination, 2)?;
    if destinations.is_empty() {
        return Err(anyhow!("No destination nodes found matching criteria"));
    }
    
    // Apply the rule for each combination
    match rule.link_type {
        LinkType::Link => {
            for source in &sources {
                for dest in &destinations {
                    match create_link(registry, mainloop, source.id, dest.id) {
                        Ok(link_id) => {
                            let msg = format!(
                                "Created link {} between {} (id: {}) and {} (id: {})",
                                link_id, source.name, source.id, dest.name, dest.id
                            );
                            results.push(msg);
                        }
                        Err(e) => {
                            let msg = format!(
                                "Failed to link {} to {}: {}",
                                source.name, dest.name, e
                            );
                            results.push(msg);
                        }
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
    fn test_wildcard_match() {
        assert!(wildcard_match("test*", "test123"));
        assert!(wildcard_match("*test", "mytest"));
        assert!(wildcard_match("*test*", "myteststring"));
        assert!(wildcard_match("test", "test"));
        assert!(!wildcard_match("test", "test123"));
        assert!(wildcard_match("node.*", "node.input"));
    }
}
