use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use tracing::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::linker::LinkRule;

/// Volume rule for devices and sinks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeRule {
    /// Human-readable name for this rule
    pub name: String,
    
    /// Object matching criteria (key-value pairs, supports regex)
    /// Works for both devices and sinks/nodes
    pub object: HashMap<String, String>,
    
    /// Volume to set (0.0 - 2.0, where 1.0 = 100%)
    pub volume: f32,
    
    /// Use state file instead of config volume if available
    #[serde(default)]
    pub use_state_file: bool,
}

/// Get the path to the user config file
fn get_user_config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|mut path| {
        path.push("pipewire-api");
        path.push("link-rules.conf");
        path
    })
}

/// Get the path to the system config file
fn get_system_config_path() -> PathBuf {
    PathBuf::from("/etc/pipewire-api/link-rules.conf")
}

/// Get the path to the user volumes config file
fn get_user_volumes_path() -> Option<PathBuf> {
    dirs::config_dir().map(|mut path| {
        path.push("pipewire-api");
        path.push("volume.conf");
        path
    })
}

/// Get the path to the system volumes config file
fn get_system_volumes_path() -> PathBuf {
    PathBuf::from("/etc/pipewire-api/volume.conf")
}

/// Load link rules from a JSON configuration file
pub fn load_link_rules_from_file(path: &PathBuf) -> Result<Vec<LinkRule>> {
    debug!("Attempting to load link rules from: {}", path.display());
    
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;
    
    let rules: Vec<LinkRule> = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse config file: {}", path.display()))?;
    
    info!("Loaded {} link rule(s) from {}", rules.len(), path.display());
    Ok(rules)
}

/// Load all link rules from available configuration files
/// 
/// Loads rules from (in order of precedence):
/// 1. User config: ~/.config/pipewire-api/link-rules.conf (highest priority)
/// 2. System config: /etc/pipewire-api/link-rules.conf (fallback)
/// 
/// Returns all rules found from both locations
pub fn load_all_link_rules() -> Vec<LinkRule> {
    let mut all_rules = Vec::new();
    
    // Try user config first (highest priority)
    if let Some(user_path) = get_user_config_path() {
        if user_path.exists() {
            match load_link_rules_from_file(&user_path) {
                Ok(rules) => {
                    info!("Loaded {} rule(s) from user config", rules.len());
                    all_rules.extend(rules);
                }
                Err(e) => {
                    warn!("Failed to load user config: {}", e);
                }
            }
        } else {
            debug!("User config file does not exist: {}", user_path.display());
        }
    }
    
    // Try system config (fallback if user config doesn't exist or is empty)
    let system_path = get_system_config_path();
    if system_path.exists() {
        match load_link_rules_from_file(&system_path) {
            Ok(rules) => {
                info!("Loaded {} rule(s) from system config", rules.len());
                all_rules.extend(rules);
            }
            Err(e) => {
                warn!("Failed to load system config: {}", e);
            }
        }
    } else {
        debug!("System config file does not exist: {}", system_path.display());
    }
    
    if all_rules.is_empty() {
        info!("No link rules loaded from config files");
    }
    
    all_rules
}

/// Load volume rules from a JSON configuration file
pub fn load_volumes_from_file(path: &PathBuf) -> Result<Vec<VolumeRule>> {
    debug!("Attempting to load volume rules from: {}", path.display());
    
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;
    
    let rules: Vec<VolumeRule> = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse config file: {}", path.display()))?;
    
    info!("Loaded {} volume rule(s) from {}", rules.len(), path.display());
    Ok(rules)
}

/// Load all volume rules from available configuration files
/// 
/// Loads rules from (in order of precedence):
/// 1. User config: ~/.config/pipewire-api/volume.conf (highest priority)
/// 2. System config: /etc/pipewire-api/volume.conf (fallback)
/// 
/// Returns all rules found from both locations
pub fn load_all_volume_rules() -> Vec<VolumeRule> {
    let mut all_rules = Vec::new();
    
    // Try user config first (highest priority)
    if let Some(user_path) = get_user_volumes_path() {
        if user_path.exists() {
            match load_volumes_from_file(&user_path) {
                Ok(rules) => {
                    info!("Loaded {} volume rule(s) from user config", rules.len());
                    all_rules.extend(rules);
                }
                Err(e) => {
                    warn!("Failed to load user volumes config: {}", e);
                }
            }
        } else {
            debug!("User volumes config file does not exist: {}", user_path.display());
        }
    }
    
    // Try system config (fallback)
    let system_path = get_system_volumes_path();
    if system_path.exists() {
        match load_volumes_from_file(&system_path) {
            Ok(rules) => {
                info!("Loaded {} volume rule(s) from system config", rules.len());
                all_rules.extend(rules);
            }
            Err(e) => {
                warn!("Failed to load system volumes config: {}", e);
            }
        }
    } else {
        debug!("System volumes config file does not exist: {}", system_path.display());
    }
    
    if all_rules.is_empty() {
        info!("No volume rules loaded from config files");
    }
    
    all_rules
}

/// Volume state entry for saving current volumes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeState {
    pub name: String,
    pub volume: f32,
}

/// Get the path to the volume state file
fn get_volume_state_path() -> Option<PathBuf> {
    dirs::home_dir().map(|mut path| {
        path.push(".state");
        path.push("pipewire-api");
        path.push("volume.state");
        path
    })
}

/// Load volume state from file
pub fn load_volume_state() -> HashMap<String, f32> {
    let mut state = HashMap::new();
    
    if let Some(state_path) = get_volume_state_path() {
        if state_path.exists() {
            match fs::read_to_string(&state_path) {
                Ok(content) => {
                    match serde_json::from_str::<Vec<VolumeState>>(&content) {
                        Ok(entries) => {
                            for entry in entries {
                                state.insert(entry.name, entry.volume);
                            }
                            debug!("Loaded {} volume state(s) from {}", state.len(), state_path.display());
                        }
                        Err(e) => {
                            warn!("Failed to parse volume state file: {}", e);
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to read volume state file: {}", e);
                }
            }
        } else {
            debug!("Volume state file does not exist: {}", state_path.display());
        }
    }
    
    state
}

/// Save volume state to file
pub fn save_volume_state(states: Vec<VolumeState>) -> Result<()> {
    if let Some(state_path) = get_volume_state_path() {
        // Create directory if it doesn't exist
        if let Some(parent) = state_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create state directory: {}", parent.display()))?;
        }
        
        let content = serde_json::to_string_pretty(&states)
            .with_context(|| "Failed to serialize volume state")?;
        
        fs::write(&state_path, content)
            .with_context(|| format!("Failed to write volume state file: {}", state_path.display()))?;
        
        info!("Saved {} volume state(s) to {}", states.len(), state_path.display());
        Ok(())
    } else {
        Err(anyhow::anyhow!("Could not determine volume state path"))
    }
}

/// Save a single volume state
pub fn save_single_volume_state(name: String, volume: f32) -> Result<()> {
    // Load existing state
    let state_path = get_volume_state_path()
        .ok_or_else(|| anyhow::anyhow!("Could not determine volume state path"))?;
    
    let mut states: Vec<VolumeState> = if state_path.exists() {
        let content = fs::read_to_string(&state_path)?;
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        Vec::new()
    };
    
    // Update or add the volume
    if let Some(existing) = states.iter_mut().find(|s| s.name == name) {
        existing.volume = volume;
    } else {
        states.push(VolumeState { name, volume });
    }
    
    save_volume_state(states)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_config_paths() {
        let system_path = get_system_config_path();
        assert_eq!(system_path.to_str().unwrap(), "/etc/pipewire-api/link-rules.conf");
        
        if let Some(user_path) = get_user_config_path() {
            assert!(user_path.to_str().unwrap().contains("pipewire-api/link-rules.conf"));
        }
    }
}
