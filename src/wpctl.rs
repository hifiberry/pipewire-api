//! Volume control via wpctl commands
//! 
//! This module provides volume control by wrapping wpctl commands,
//! which is simpler and more reliable than direct PipeWire API calls.

use std::process::Command;
use regex::Regex;
use serde::{Deserialize, Serialize};

/// Information about a volume-controllable object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeInfo {
    pub id: u32,
    pub name: String,
    pub object_type: String,
    pub volume: f32,
}

/// Parse wpctl status output to find all objects with volume control
/// 
/// Looks for lines like:
///   81. Built-in Audio Stereo               [vol: 0.50]
///   38. effect_input.proc                   [vol: 1.00]
pub fn list_volumes() -> Result<Vec<VolumeInfo>, String> {
    let output = Command::new("wpctl")
        .arg("status")
        .output()
        .map_err(|e| format!("Failed to run wpctl status: {}", e))?;
    
    if !output.status.success() {
        return Err(format!("wpctl status failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_wpctl_status(&stdout)
}

/// Parse wpctl status output
fn parse_wpctl_status(status: &str) -> Result<Vec<VolumeInfo>, String> {
    let mut volumes = Vec::new();
    
    // Relaxed regex: find lines with [vol: X.XX] and extract ID and name
    // Matches: "81. Built-in Audio Stereo [vol: 0.50]"
    // The ID is any number followed by a dot, name is everything before [vol:
    let re = Regex::new(r"(\d+)\.\s+(.+?)\s+\[vol:\s*([\d.]+)").unwrap();
    
    let mut current_section = String::new();
    
    for line in status.lines() {
        // Track which section we're in (Sinks, Sources, Filters, etc.)
        if line.contains("Sinks:") {
            current_section = "sink".to_string();
        } else if line.contains("Sources:") {
            current_section = "source".to_string();
        } else if line.contains("Filters:") {
            current_section = "filter".to_string();
        } else if line.contains("Devices:") {
            current_section = "device".to_string();
        } else if line.contains("Streams:") {
            current_section = "stream".to_string();
        }
        
        // Try to match volume lines
        if let Some(caps) = re.captures(line) {
            let id: u32 = caps.get(1).unwrap().as_str().parse().unwrap_or(0);
            let name = caps.get(2).unwrap().as_str().trim().to_string();
            let volume: f32 = caps.get(3).unwrap().as_str().parse().unwrap_or(1.0);
            
            // Determine object type from section or line content
            let object_type = if !current_section.is_empty() {
                current_section.clone()
            } else if line.contains("Audio/Sink") {
                "sink".to_string()
            } else if line.contains("Audio/Source") {
                "source".to_string()
            } else {
                "unknown".to_string()
            };
            
            volumes.push(VolumeInfo {
                id,
                name,
                object_type,
                volume,
            });
        }
    }
    
    Ok(volumes)
}

/// Get volume for a specific object by ID
pub fn get_volume(id: u32) -> Result<VolumeInfo, String> {
    // First get the volume value
    let output = Command::new("wpctl")
        .args(["get-volume", &id.to_string()])
        .output()
        .map_err(|e| format!("Failed to run wpctl get-volume: {}", e))?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Check for "not found" in both stdout and stderr (wpctl returns it in stdout with exit code 0)
    if stdout.contains("not found") || stderr.contains("not found") {
        return Err(format!("Object {} not found", id));
    }
    
    if !output.status.success() {
        return Err(format!("wpctl get-volume failed: {}", stderr));
    }
    
    // Parse "Volume: 0.50" or "Volume: 0.50 [MUTED]"
    let volume = parse_volume_output(&stdout)?;
    
    // Get name and type from wpctl status
    let (name, object_type) = get_object_info(id)?;
    
    Ok(VolumeInfo {
        id,
        name,
        object_type,
        volume,
    })
}

/// Parse wpctl get-volume output
fn parse_volume_output(output: &str) -> Result<f32, String> {
    let re = Regex::new(r"Volume:\s*([\d.]+)").unwrap();
    
    if let Some(caps) = re.captures(output) {
        caps.get(1)
            .unwrap()
            .as_str()
            .parse()
            .map_err(|e| format!("Failed to parse volume: {}", e))
    } else {
        Err("Could not parse volume output".to_string())
    }
}

/// Get object name and type from wpctl status
fn get_object_info(id: u32) -> Result<(String, String), String> {
    let output = Command::new("wpctl")
        .arg("status")
        .output()
        .map_err(|e| format!("Failed to run wpctl status: {}", e))?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Look for the ID in the status output - relaxed pattern
    let id_pattern = format!(r"{}\.\s+(.+?)(?:\s+\[|$)", id);
    let re = Regex::new(&id_pattern).unwrap();
    
    let mut current_section = "unknown".to_string();
    
    for line in stdout.lines() {
        // Track section
        if line.contains("Sinks:") {
            current_section = "sink".to_string();
        } else if line.contains("Sources:") {
            current_section = "source".to_string();
        } else if line.contains("Filters:") {
            current_section = "filter".to_string();
        } else if line.contains("Devices:") {
            current_section = "device".to_string();
        } else if line.contains("Streams:") {
            current_section = "stream".to_string();
        }
        
        if let Some(caps) = re.captures(line) {
            let name = caps.get(1).unwrap().as_str().trim().to_string();
            return Ok((name, current_section));
        }
    }
    
    Err(format!("Object {} not found in wpctl status", id))
}

/// Set volume for a specific object by ID
pub fn set_volume(id: u32, volume: f32) -> Result<f32, String> {
    // Clamp volume to reasonable range (0.0 to 2.0)
    let volume = volume.max(0.0).min(2.0);
    
    let output = Command::new("wpctl")
        .args(["set-volume", &id.to_string(), &format!("{:.2}", volume)])
        .output()
        .map_err(|e| format!("Failed to run wpctl set-volume: {}", e))?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Check for "not found" in both stdout and stderr
    if stdout.contains("not found") || stderr.contains("not found") {
        return Err(format!("Object {} not found", id));
    }
    
    if !output.status.success() {
        return Err(format!("wpctl set-volume failed: {}", stderr));
    }
    
    Ok(volume)
}

/// Information about a default audio node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultNodeInfo {
    pub id: u32,
    pub name: String,
    pub description: Option<String>,
    pub media_class: Option<String>,
}

/// Get default audio sink information
pub fn get_default_sink() -> Result<DefaultNodeInfo, String> {
    get_default_node("@DEFAULT_AUDIO_SINK@")
}

/// Get default audio source information
pub fn get_default_source() -> Result<DefaultNodeInfo, String> {
    get_default_node("@DEFAULT_AUDIO_SOURCE@")
}

/// Get information about a default node using wpctl inspect
fn get_default_node(selector: &str) -> Result<DefaultNodeInfo, String> {
    let output = Command::new("wpctl")
        .args(["inspect", selector])
        .output()
        .map_err(|e| format!("Failed to run wpctl inspect: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("wpctl inspect failed: {}", stderr));
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_wpctl_inspect(&stdout)
}

/// Parse wpctl inspect output to extract node information
fn parse_wpctl_inspect(output: &str) -> Result<DefaultNodeInfo, String> {
    let mut id: Option<u32> = None;
    let mut name: Option<String> = None;
    let mut description: Option<String> = None;
    let mut media_class: Option<String> = None;
    
    for line in output.lines() {
        let line = line.trim();
        
        // First line has the id: "id 38, type PipeWire:Interface:Node"
        if line.starts_with("id ") {
            if let Some(id_str) = line.split(',').next() {
                if let Some(num_str) = id_str.strip_prefix("id ") {
                    id = num_str.trim().parse().ok();
                }
            }
        }
        
        // Parse key = "value" lines
        if let Some((key, value)) = line.split_once(" = ") {
            let key = key.trim().trim_start_matches("* ");
            let value = value.trim().trim_matches('"');
            
            match key {
                "node.name" => name = Some(value.to_string()),
                "node.description" => description = Some(value.to_string()),
                "media.class" => media_class = Some(value.to_string()),
                _ => {}
            }
        }
    }
    
    Ok(DefaultNodeInfo {
        id: id.ok_or("Could not find node id")?,
        name: name.ok_or("Could not find node.name")?,
        description,
        media_class,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_wpctl_status() {
        let status = r#"
PipeWire 'pipewire-0' [1.4.2, user@host, cookie:123]
 └─ Clients:
        34. WirePlumber

Audio
 ├─ Devices:
 │      56. Built-in Audio                      [alsa]
 │  
 ├─ Sinks:
 │      81. Built-in Audio Stereo               [vol: 0.50]
 │  
 ├─ Sources:
 │  
 ├─ Filters:
 │  *   38. effect_input.proc                   [vol: 1.00]
 │      44. speakereq2x2                        [vol: 0.75]
 │  
 └─ Streams:
"#;
        
        let volumes = parse_wpctl_status(status).unwrap();
        
        assert_eq!(volumes.len(), 3);
        
        // Check sink
        let sink = volumes.iter().find(|v| v.id == 81).unwrap();
        assert_eq!(sink.name, "Built-in Audio Stereo");
        assert_eq!(sink.object_type, "sink");
        assert!((sink.volume - 0.50).abs() < 0.01);
        
        // Check filter with default marker
        let filter = volumes.iter().find(|v| v.id == 38).unwrap();
        assert_eq!(filter.name, "effect_input.proc");
        assert_eq!(filter.object_type, "filter");
        assert!((filter.volume - 1.0).abs() < 0.01);
    }
    
    #[test]
    fn test_parse_volume_output() {
        assert!((parse_volume_output("Volume: 0.50").unwrap() - 0.50).abs() < 0.01);
        assert!((parse_volume_output("Volume: 1.00 [MUTED]").unwrap() - 1.0).abs() < 0.01);
        assert!((parse_volume_output("Volume: 0.75\n").unwrap() - 0.75).abs() < 0.01);
    }
    
    #[test]
    fn test_parse_wpctl_inspect_full() {
        let output = r#"id 38, type PipeWire:Interface:Node
    adapt.follower.spa-node = ""
    audio.channels = "2"
    audio.position = "[ FL FR ]"
  * client.id = "35"
    clock.quantum-limit = "8192"
  * factory.id = "18"
    library.name = "audioconvert/libspa-audioconvert"
  * media.class = "Audio/Sink"
    media.name = "EQ + Balance Sink"
    node.autoconnect = "true"
  * node.description = "EQ + Balance Sink"
    node.driver-id = "81"
    node.group = "filter-chain-1599212-28"
    node.link-group = "filter-chain-1599212-28"
  * node.name = "effect_input.proc"
    node.virtual = "true"
"#;
        
        let info = parse_wpctl_inspect(output).unwrap();
        assert_eq!(info.id, 38);
        assert_eq!(info.name, "effect_input.proc");
        assert_eq!(info.description, Some("EQ + Balance Sink".to_string()));
        assert_eq!(info.media_class, Some("Audio/Sink".to_string()));
    }
    
    #[test]
    fn test_parse_wpctl_inspect_minimal() {
        let output = r#"id 81, type PipeWire:Interface:Node
  * node.name = "alsa_output.platform-hdmi"
"#;
        
        let info = parse_wpctl_inspect(output).unwrap();
        assert_eq!(info.id, 81);
        assert_eq!(info.name, "alsa_output.platform-hdmi");
        assert_eq!(info.description, None);
        assert_eq!(info.media_class, None);
    }
    
    #[test]
    fn test_parse_wpctl_inspect_with_description_and_class() {
        let output = r#"id 42, type PipeWire:Interface:Node
  * media.class = "Audio/Source/Virtual"
  * node.description = "RIAA Filter Input"
  * node.name = "riaa"
"#;
        
        let info = parse_wpctl_inspect(output).unwrap();
        assert_eq!(info.id, 42);
        assert_eq!(info.name, "riaa");
        assert_eq!(info.description, Some("RIAA Filter Input".to_string()));
        assert_eq!(info.media_class, Some("Audio/Source/Virtual".to_string()));
    }
    
    #[test]
    fn test_parse_wpctl_inspect_missing_name() {
        let output = r#"id 99, type PipeWire:Interface:Node
  * media.class = "Audio/Sink"
"#;
        
        let result = parse_wpctl_inspect(output);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("node.name"));
    }
    
    #[test]
    fn test_parse_wpctl_inspect_missing_id() {
        let output = r#"type PipeWire:Interface:Node
  * node.name = "test_node"
"#;
        
        let result = parse_wpctl_inspect(output);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("node id"));
    }
}
