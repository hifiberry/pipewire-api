//! Parameter rules configuration and execution
//!
//! Loads parameter rules from param-rules.conf and applies them to nodes on startup

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tracing::{debug, error, info, warn};

use crate::pwcli;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMatcher {
    #[serde(rename = "node.name")]
    pub node_name: Option<String>,
    #[serde(rename = "object.path")]
    pub object_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamRule {
    pub name: String,
    pub node: NodeMatcher,
    pub parameters: HashMap<String, serde_json::Value>,
    #[serde(default = "default_true")]
    pub set_at_startup: bool,
    #[serde(default = "default_info_level")]
    pub info_level: String,
    #[serde(default = "default_error_level")]
    pub error_level: String,
}

fn default_true() -> bool {
    true
}

fn default_info_level() -> String {
    "info".to_string()
}

fn default_error_level() -> String {
    "error".to_string()
}

/// Load parameter rules from configuration file
pub fn load_param_rules(config_path: &Path) -> Result<Vec<ParamRule>, String> {
    if !config_path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(config_path)
        .map_err(|e| format!("Failed to read param rules config: {}", e))?;

    let rules: Vec<ParamRule> = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse param rules config: {}", e))?;

    info!("Loaded {} parameter rule(s) from {:?}", rules.len(), config_path);
    Ok(rules)
}

/// Check if a node matches the matcher criteria
fn node_matches(node: &pwcli::PwObject, matcher: &NodeMatcher) -> bool {
    // Check node.name pattern
    if let Some(pattern) = &matcher.node_name {
        if let Some(node_name) = node.properties.get("node.name") {
            if let Ok(re) = regex::Regex::new(pattern) {
                if !re.is_match(node_name) {
                    return false;
                }
            } else {
                warn!("Invalid regex pattern in node matcher: {}", pattern);
                return false;
            }
        } else {
            return false;
        }
    }

    // Check object.path pattern
    if let Some(pattern) = &matcher.object_path {
        if let Some(object_path) = node.properties.get("object.path") {
            if let Ok(re) = regex::Regex::new(pattern) {
                if !re.is_match(object_path) {
                    return false;
                }
            } else {
                warn!("Invalid regex pattern in object matcher: {}", pattern);
                return false;
            }
        } else {
            return false;
        }
    }

    true
}

/// Apply parameter rules to nodes
pub async fn apply_param_rules(rules: &[ParamRule]) -> Result<(), String> {
    use std::process::Command;
    use crate::parameters::ParameterValue;
    
    if rules.is_empty() {
        debug!("No parameter rules to apply");
        return Ok(());
    }

    // Get all nodes
    let objects = pwcli::list_nodes()
        .map_err(|e| format!("Failed to get PipeWire nodes: {}", e))?;

    for rule in rules {
        if !rule.set_at_startup {
            continue;
        }

        debug!("Processing parameter rule: {}", rule.name);

        // Find matching nodes
        let matching_nodes: Vec<&pwcli::PwObject> = objects
            .iter()
            .filter(|obj| node_matches(obj, &rule.node))
            .collect();

        if matching_nodes.is_empty() {
            match rule.error_level.as_str() {
                "error" => error!("No nodes found matching rule: {}", rule.name),
                "warn" => warn!("No nodes found matching rule: {}", rule.name),
                _ => debug!("No nodes found matching rule: {}", rule.name),
            }
            continue;
        }

        // Apply parameters to each matching node
        for node in matching_nodes {
            let node_name = node.properties.get("node.name")
                .map(|s| s.as_str())
                .unwrap_or("unknown");

            match rule.info_level.as_str() {
                "info" => info!("Applying parameters to node: {} (ID: {})", node_name, node.id),
                "debug" => debug!("Applying parameters to node: {} (ID: {})", node_name, node.id),
                _ => {}
            }

            // Convert parameters to ParameterValue format
            let mut params = HashMap::new();
            for (param_name, param_value) in &rule.parameters {
                let value = match param_value {
                    serde_json::Value::Number(n) => {
                        if let Some(i) = n.as_i64() {
                            ParameterValue::Int(i as i32)
                        } else {
                            ParameterValue::Float(n.as_f64().unwrap_or(0.0) as f32)
                        }
                    }
                    serde_json::Value::Bool(b) => ParameterValue::Bool(*b),
                    serde_json::Value::String(s) => ParameterValue::String(s.clone()),
                    _ => {
                        warn!("Unsupported parameter value type for {}: {:?}", param_name, param_value);
                        continue;
                    }
                };
                params.insert(param_name.clone(), value);
            }

            // Build array format for params struct
            let mut params_array = Vec::new();
            for (key, value) in params {
                params_array.push(serde_json::Value::String(key.clone()));
                
                let json_value = match value {
                    ParameterValue::Bool(b) => serde_json::Value::Bool(b),
                    ParameterValue::Int(i) => serde_json::Value::Number(i.into()),
                    ParameterValue::Float(f) => {
                        serde_json::Number::from_f64(f as f64)
                            .map(serde_json::Value::Number)
                            .unwrap_or(serde_json::Value::Null)
                    },
                    ParameterValue::String(s) => serde_json::Value::String(s),
                };
                params_array.push(json_value);
            }

            // Wrap in params property
            let json = serde_json::json!({ "params": params_array });
            let json_str = json.to_string();

            // Set parameters via pw-cli
            let output = Command::new("pw-cli")
                .args(&["set-param", &node.id.to_string(), "Props", &json_str])
                .output()
                .map_err(|e| format!("Failed to execute pw-cli: {}", e))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                match rule.error_level.as_str() {
                    "error" => error!("Failed to set parameters on {}: {}", node_name, stderr),
                    "warn" => warn!("Failed to set parameters on {}: {}", node_name, stderr),
                    _ => debug!("Failed to set parameters on {}: {}", node_name, stderr),
                }
            } else {
                debug!("Successfully set parameters on {}", node_name);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_param_rule_deserialization() {
        let json = r#"[
            {
                "name": "Enable speakereq",
                "node": {
                    "node.name": "^speakereq[0-9]x[0-9]$"
                },
                "parameters": {
                    "Enable": 1
                },
                "set_at_startup": true,
                "info_level": "info",
                "error_level": "error"
            }
        ]"#;

        let rules: Result<Vec<ParamRule>, _> = serde_json::from_str(json);
        assert!(rules.is_ok());
        
        let rules = rules.unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].name, "Enable speakereq");
        assert!(rules[0].set_at_startup);
        assert_eq!(rules[0].info_level, "info");
        assert_eq!(rules[0].error_level, "error");
        assert_eq!(rules[0].parameters.len(), 1);
        assert_eq!(rules[0].parameters.get("Enable").unwrap().as_i64().unwrap(), 1);
    }

    #[test]
    fn test_param_rule_defaults() {
        let json = r#"[
            {
                "name": "Test rule",
                "node": {
                    "node.name": "test"
                },
                "parameters": {
                    "Volume": 0.5
                }
            }
        ]"#;

        let rules: Vec<ParamRule> = serde_json::from_str(json).unwrap();
        assert_eq!(rules[0].set_at_startup, true); // default
        assert_eq!(rules[0].info_level, "info"); // default
        assert_eq!(rules[0].error_level, "error"); // default
    }

    #[test]
    fn test_node_matches_by_name() {
        let mut properties = HashMap::new();
        properties.insert("node.name".to_string(), "speakereq2x2".to_string());
        
        let node = pwcli::PwObject {
            id: 42,
            object_type: "Node".to_string(),
            properties,
        };

        // Test exact regex match
        let matcher = NodeMatcher {
            node_name: Some("^speakereq2x2$".to_string()),
            object_path: None,
        };
        assert!(node_matches(&node, &matcher));

        // Test pattern match
        let matcher = NodeMatcher {
            node_name: Some("^speakereq[0-9]x[0-9]$".to_string()),
            object_path: None,
        };
        assert!(node_matches(&node, &matcher));

        // Test non-match
        let matcher = NodeMatcher {
            node_name: Some("^filter".to_string()),
            object_path: None,
        };
        assert!(!node_matches(&node, &matcher));
    }

    #[test]
    fn test_node_matches_by_path() {
        let mut properties = HashMap::new();
        properties.insert("object.path".to_string(), "filter:speakereq".to_string());
        
        let node = pwcli::PwObject {
            id: 42,
            object_type: "Node".to_string(),
            properties,
        };

        let matcher = NodeMatcher {
            node_name: None,
            object_path: Some("filter:.*".to_string()),
        };
        assert!(node_matches(&node, &matcher));

        let matcher = NodeMatcher {
            node_name: None,
            object_path: Some("^device:.*".to_string()),
        };
        assert!(!node_matches(&node, &matcher));
    }

    #[test]
    fn test_node_matches_both_criteria() {
        let mut properties = HashMap::new();
        properties.insert("node.name".to_string(), "speakereq2x2".to_string());
        properties.insert("object.path".to_string(), "filter:speakereq".to_string());
        
        let node = pwcli::PwObject {
            id: 42,
            object_type: "Node".to_string(),
            properties,
        };

        // Both must match
        let matcher = NodeMatcher {
            node_name: Some("^speakereq.*".to_string()),
            object_path: Some("filter:.*".to_string()),
        };
        assert!(node_matches(&node, &matcher));

        // One doesn't match
        let matcher = NodeMatcher {
            node_name: Some("^speakereq.*".to_string()),
            object_path: Some("device:.*".to_string()),
        };
        assert!(!node_matches(&node, &matcher));
    }

    #[test]
    fn test_node_matches_no_criteria() {
        let mut properties = HashMap::new();
        properties.insert("node.name".to_string(), "anything".to_string());
        
        let node = pwcli::PwObject {
            id: 42,
            object_type: "Node".to_string(),
            properties,
        };

        // Empty matcher matches everything
        let matcher = NodeMatcher {
            node_name: None,
            object_path: None,
        };
        assert!(node_matches(&node, &matcher));
    }

    #[test]
    fn test_load_param_rules_from_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let content = r#"[
            {
                "name": "Enable speakereq",
                "node": {
                    "node.name": "^speakereq[0-9]x[0-9]$"
                },
                "parameters": {
                    "Enable": 1,
                    "Volume": 0.8
                }
            },
            {
                "name": "RIAA gain",
                "node": {
                    "node.name": "riaa"
                },
                "parameters": {
                    "Gain": 0.5
                }
            }
        ]"#;
        
        temp_file.write_all(content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let result = load_param_rules(&temp_file.path().to_path_buf());
        assert!(result.is_ok());
        
        let rules = result.unwrap();
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].name, "Enable speakereq");
        assert_eq!(rules[0].parameters.len(), 2);
        assert_eq!(rules[1].name, "RIAA gain");
        assert_eq!(rules[1].parameters.len(), 1);
    }

    #[test]
    fn test_load_param_rules_nonexistent_file() {
        let path = std::path::PathBuf::from("/nonexistent/file.conf");
        let result = load_param_rules(&path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn test_load_param_rules_invalid_json() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"not valid json").unwrap();
        temp_file.flush().unwrap();

        let result = load_param_rules(&temp_file.path().to_path_buf());
        assert!(result.is_err());
    }

    #[test]
    fn test_param_value_conversion_number() {
        let json = r#"{"Volume": 0.75}"#;
        let params: HashMap<String, serde_json::Value> = serde_json::from_str(json).unwrap();
        
        let value = params.get("Volume").unwrap();
        let value_float = value.as_f64().unwrap() as f32;
        assert!((value_float - 0.75).abs() < 0.001);
    }

    #[test]
    fn test_param_value_conversion_bool() {
        let json = r#"{"Enable": true}"#;
        let params: HashMap<String, serde_json::Value> = serde_json::from_str(json).unwrap();
        
        let value = params.get("Enable").unwrap();
        let value_float = if value.as_bool().unwrap() { 1.0 } else { 0.0 };
        assert_eq!(value_float, 1.0);
    }

    #[test]
    fn test_param_value_conversion_integer() {
        let json = r#"{"Count": 42}"#;
        let params: HashMap<String, serde_json::Value> = serde_json::from_str(json).unwrap();
        
        let value = params.get("Count").unwrap();
        let value_float = value.as_f64().unwrap() as f32;
        assert_eq!(value_float, 42.0);
    }

    #[test]
    fn test_regex_pattern_validation() {
        let mut properties = HashMap::new();
        properties.insert("node.name".to_string(), "test123".to_string());
        
        let node = pwcli::PwObject {
            id: 42,
            object_type: "Node".to_string(),
            properties,
        };

        // Valid regex patterns
        assert!(node_matches(&node, &NodeMatcher {
            node_name: Some("test[0-9]+".to_string()),
            object_path: None,
        }));

        assert!(node_matches(&node, &NodeMatcher {
            node_name: Some("^test.*".to_string()),
            object_path: None,
        }));

        assert!(node_matches(&node, &NodeMatcher {
            node_name: Some(".*123$".to_string()),
            object_path: None,
        }));
    }
}
