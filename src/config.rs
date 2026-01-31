use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use tracing::{debug, info, warn};

use crate::linker::LinkRule;

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
/// 1. System config: /etc/pipewire-api/link-rules.conf
/// 2. User config: ~/.config/pipewire-api/link-rules.conf
/// 
/// User config takes precedence if both exist
pub fn load_all_link_rules() -> Vec<LinkRule> {
    let mut all_rules = Vec::new();
    
    // Try system config first
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
    
    // Try user config (takes precedence)
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
    
    if all_rules.is_empty() {
        info!("No link rules loaded from config files");
    }
    
    all_rules
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
