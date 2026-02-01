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
    // Clamp volume to reasonable range (0.0 to 1.5)
    let volume = volume.max(0.0).min(1.5);
    
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
}
