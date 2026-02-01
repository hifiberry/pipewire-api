use anyhow::Result;
use regex::Regex;
use std::collections::HashMap;
use tracing::{debug, info, warn, error};

use crate::config::VolumeRule;

/// Apply volume rules on startup for both devices and sinks
pub fn apply_volume_rules(rules: Vec<VolumeRule>) -> Result<()> {
    if rules.is_empty() {
        info!("No volume rules to apply");
        return Ok(());
    }
    
    info!("Applying {} volume rule(s)", rules.len());
    
    // Get all PipeWire objects using pwcli
    let objects = crate::pwcli::list_objects(None)
        .map_err(|e| anyhow::anyhow!("Failed to list objects: {}", e))?;
    
    // Load volume state file
    let volume_state = crate::config::load_volume_state();
    if !volume_state.is_empty() {
        info!("Loaded {} volume(s) from state file", volume_state.len());
    }
    
    // Apply rules to matching objects
    for rule in &rules {
        debug!("Processing rule: {}", rule.name);
        
        // Compile regex patterns
        let mut regex_patterns: HashMap<String, Regex> = HashMap::new();
        for (key, pattern) in &rule.object {
            match Regex::new(pattern) {
                Ok(re) => {
                    regex_patterns.insert(key.clone(), re);
                }
                Err(e) => {
                    warn!("Invalid regex pattern '{}' in rule '{}': {}", pattern, rule.name, e);
                    continue;
                }
            }
        }
        
        // Find matching objects
        for object in &objects {
            let mut matches = true;
            
            for (key, regex) in &regex_patterns {
                if let Some(value) = object.properties.get(key) {
                    if !regex.is_match(value) {
                        matches = false;
                        break;
                    }
                } else {
                    matches = false;
                    break;
                }
            }
            
            if matches {
                // Get object name for logging and state file lookup
                let object_name = object.properties.get("node.name")
                    .or_else(|| object.properties.get("device.name"))
                    .or_else(|| object.properties.get("node.description"))
                    .or_else(|| object.properties.get("device.description"))
                    .map(|s| s.as_str())
                    .unwrap_or("unknown");
                
                // Determine volume to apply
                let volume_to_apply = if rule.use_state_file {
                    if let Some(state_volume) = volume_state.get(object_name) {
                        info!("Using state file volume {:.2} for {} {} ({})", 
                              state_volume, object.object_type, object.id, object_name);
                        *state_volume
                    } else {
                        info!("Applying config volume {:.2} to {} {} ({})", 
                              rule.volume, object.object_type, object.id, object_name);
                        rule.volume
                    }
                } else {
                    info!("Applying config volume {:.2} to {} {} ({})", 
                          rule.volume, object.object_type, object.id, object_name);
                    rule.volume
                };
                
                // Set volume using wpctl
                if let Err(e) = set_volume_wpctl(object.id, volume_to_apply) {
                    error!("Failed to set volume for {} {}: {}", object.object_type, object.id, e);
                } else {
                    debug!("Successfully set volume for {} {}", object.object_type, object.id);
                }
            }
        }
    }
    
    Ok(())
}

/// Set volume using wpctl command
fn set_volume_wpctl(id: u32, volume: f32) -> Result<()> {
    use std::process::Command;
    
    // wpctl expects volume as a percentage (0.0 to 1.0)
    // The volume value is already in this format
    let volume_str = format!("{:.4}", volume);
    
    let output = Command::new("wpctl")
        .args(["set-volume", &id.to_string(), &volume_str])
        .output()?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("wpctl set-volume failed: {}", stderr));
    }
    
    Ok(())
}
