//! Parser and executor for pw-link command
//!
//! This module provides a simple interface to PipeWire links by using
//! the pw-link command line tool.

use std::process::Command;
use regex::Regex;
use serde::{Deserialize, Serialize};

/// A PipeWire port as returned by pw-link
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PwPort {
    pub id: u32,
    pub name: String,
    pub node_name: String,
    pub port_name: String,
}

impl PwPort {
    /// Parse a port from "node_name:port_name" format
    fn from_full_name(id: u32, full_name: &str) -> Self {
        let (node_name, port_name) = if let Some(pos) = full_name.find(':') {
            (full_name[..pos].to_string(), full_name[pos + 1..].to_string())
        } else {
            (full_name.to_string(), String::new())
        };
        
        PwPort {
            id,
            name: full_name.to_string(),
            node_name,
            port_name,
        }
    }
}

/// A PipeWire link as returned by pw-link
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PwLink {
    pub id: u32,
    pub output_port_id: u32,
    pub output_port_name: String,
    pub input_port_id: u32,
    pub input_port_name: String,
}

/// Direction for listing ports
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortDirection {
    Output,
    Input,
}

/// List output ports
pub fn list_output_ports() -> Result<Vec<PwPort>, String> {
    list_ports(PortDirection::Output)
}

/// List input ports
pub fn list_input_ports() -> Result<Vec<PwPort>, String> {
    list_ports(PortDirection::Input)
}

/// List ports in a given direction
pub fn list_ports(direction: PortDirection) -> Result<Vec<PwPort>, String> {
    let mut cmd = Command::new("pw-link");
    cmd.arg("-I");
    
    match direction {
        PortDirection::Output => cmd.arg("-o"),
        PortDirection::Input => cmd.arg("-i"),
    };
    
    let output = cmd
        .output()
        .map_err(|e| format!("Failed to run pw-link: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("pw-link failed: {}", stderr));
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_ports(&stdout)
}

/// Parse pw-link port output
/// Format: "  53 effect_input.proc:monitor_FL"
fn parse_ports(output: &str) -> Result<Vec<PwPort>, String> {
    let mut ports = Vec::new();
    
    // Regex: whitespace, id, whitespace, name
    let re = Regex::new(r"^\s*(\d+)\s+(.+)$")
        .map_err(|e| format!("Invalid regex: {}", e))?;
    
    for line in output.lines() {
        if let Some(caps) = re.captures(line) {
            let id: u32 = caps.get(1).unwrap().as_str().parse().unwrap_or(0);
            let name = caps.get(2).unwrap().as_str().trim();
            ports.push(PwPort::from_full_name(id, name));
        }
    }
    
    Ok(ports)
}

/// List all links
pub fn list_links() -> Result<Vec<PwLink>, String> {
    let output = Command::new("pw-link")
        .args(["-l", "-I"])
        .output()
        .map_err(|e| format!("Failed to run pw-link: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("pw-link failed: {}", stderr));
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_links(&stdout)
}

/// Parse pw-link -l -I output
/// Format:
/// ```text
///   90 effect_output.proc:output_FL
///   92   |->   82 speakereq2x2:playback_FL
/// ```
fn parse_links(output: &str) -> Result<Vec<PwLink>, String> {
    let mut links = Vec::new();
    
    // Regex for output port line: "  90 effect_output.proc:output_FL"
    let port_re = Regex::new(r"^\s*(\d+)\s+(\S+:\S+)\s*$")
        .map_err(|e| format!("Invalid port regex: {}", e))?;
    
    // Regex for link line: "  92   |->   82 speakereq2x2:playback_FL"
    let link_re = Regex::new(r"^\s*(\d+)\s+\|->\s+(\d+)\s+(\S+:\S+)\s*$")
        .map_err(|e| format!("Invalid link regex: {}", e))?;
    
    let mut current_output_id: Option<u32> = None;
    let mut current_output_name: Option<String> = None;
    
    for line in output.lines() {
        // Check if it's a link line first (more specific pattern)
        if let Some(caps) = link_re.captures(line) {
            if let (Some(out_id), Some(out_name)) = (current_output_id, &current_output_name) {
                let link_id: u32 = caps.get(1).unwrap().as_str().parse().unwrap_or(0);
                let input_id: u32 = caps.get(2).unwrap().as_str().parse().unwrap_or(0);
                let input_name = caps.get(3).unwrap().as_str().to_string();
                
                links.push(PwLink {
                    id: link_id,
                    output_port_id: out_id,
                    output_port_name: out_name.clone(),
                    input_port_id: input_id,
                    input_port_name: input_name,
                });
            }
        }
        // Check if it's a port line (skip lines with |<-)
        else if !line.contains("|<-") {
            if let Some(caps) = port_re.captures(line) {
                current_output_id = Some(caps.get(1).unwrap().as_str().parse().unwrap_or(0));
                current_output_name = Some(caps.get(2).unwrap().as_str().to_string());
            }
        }
    }
    
    Ok(links)
}

/// Create a link between two ports by name
/// 
/// # Arguments
/// * `output` - Output port name (e.g., "effect_output.proc:output_FL")
/// * `input` - Input port name (e.g., "speakereq2x2:playback_FL")
pub fn create_link(output: &str, input: &str) -> Result<(), String> {
    let output_cmd = Command::new("pw-link")
        .args([output, input])
        .output()
        .map_err(|e| format!("Failed to run pw-link: {}", e))?;
    
    if !output_cmd.status.success() {
        let stderr = String::from_utf8_lossy(&output_cmd.stderr);
        return Err(format!("Failed to create link: {}", stderr.trim()));
    }
    
    Ok(())
}

/// Create a link between two ports by ID
pub fn create_link_by_id(output_id: u32, input_id: u32) -> Result<(), String> {
    let output = Command::new("pw-link")
        .args([&output_id.to_string(), &input_id.to_string()])
        .output()
        .map_err(|e| format!("Failed to run pw-link: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to create link: {}", stderr.trim()));
    }
    
    Ok(())
}

/// Remove a link by its link ID
pub fn remove_link(link_id: u32) -> Result<(), String> {
    let output = Command::new("pw-link")
        .args(["-d", &link_id.to_string()])
        .output()
        .map_err(|e| format!("Failed to run pw-link: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to remove link: {}", stderr.trim()));
    }
    
    Ok(())
}

/// Remove a link between two ports by name
pub fn remove_link_by_name(output: &str, input: &str) -> Result<(), String> {
    let output_cmd = Command::new("pw-link")
        .args(["-d", output, input])
        .output()
        .map_err(|e| format!("Failed to run pw-link: {}", e))?;
    
    if !output_cmd.status.success() {
        let stderr = String::from_utf8_lossy(&output_cmd.stderr);
        return Err(format!("Failed to remove link: {}", stderr.trim()));
    }
    
    Ok(())
}

/// Find a port by name pattern (partial match)
pub fn find_port(direction: PortDirection, pattern: &str) -> Result<Option<PwPort>, String> {
    let ports = list_ports(direction)?;
    Ok(ports.into_iter().find(|p| p.name.contains(pattern)))
}

/// Find a port by exact name
pub fn find_port_exact(direction: PortDirection, name: &str) -> Result<Option<PwPort>, String> {
    let ports = list_ports(direction)?;
    Ok(ports.into_iter().find(|p| p.name == name))
}

/// Find a link by output and input port names
pub fn find_link(output: &str, input: &str) -> Result<Option<PwLink>, String> {
    let links = list_links()?;
    Ok(links.into_iter().find(|l| 
        l.output_port_name == output && l.input_port_name == input
    ))
}

/// Check if a link exists between two ports
pub fn link_exists(output: &str, input: &str) -> Result<bool, String> {
    find_link(output, input).map(|l| l.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_ports() {
        let output = r#"  53 effect_input.proc:monitor_FL
  55 effect_input.proc:monitor_FR
  90 effect_output.proc:output_FL
"#;
        let ports = parse_ports(output).unwrap();
        assert_eq!(ports.len(), 3);
        assert_eq!(ports[0].id, 53);
        assert_eq!(ports[0].name, "effect_input.proc:monitor_FL");
        assert_eq!(ports[0].node_name, "effect_input.proc");
        assert_eq!(ports[0].port_name, "monitor_FL");
    }
    
    #[test]
    fn test_parse_links() {
        let output = r#"  90 effect_output.proc:output_FL
  92   |->   82 speakereq2x2:playback_FL
  91 effect_output.proc:output_FR
  93   |->   84 speakereq2x2:playback_FR
  82 speakereq2x2:playback_FL
  92   |<-   90 effect_output.proc:output_FL
"#;
        let links = parse_links(output).unwrap();
        assert_eq!(links.len(), 2);
        
        assert_eq!(links[0].id, 92);
        assert_eq!(links[0].output_port_id, 90);
        assert_eq!(links[0].output_port_name, "effect_output.proc:output_FL");
        assert_eq!(links[0].input_port_id, 82);
        assert_eq!(links[0].input_port_name, "speakereq2x2:playback_FL");
        
        assert_eq!(links[1].id, 93);
        assert_eq!(links[1].output_port_id, 91);
    }
    
    #[test]
    fn test_port_from_full_name() {
        let port = PwPort::from_full_name(42, "node:port_name");
        assert_eq!(port.id, 42);
        assert_eq!(port.name, "node:port_name");
        assert_eq!(port.node_name, "node");
        assert_eq!(port.port_name, "port_name");
    }
}
