use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use std::sync::{Arc, Mutex, RwLock};
use std::collections::HashMap;
use crate::parameters::ParameterValue;
use crate::linker::LinkRule;
use crate::pwcli::PwObject;
use serde::{Deserialize, Serialize};
use tracing::info;

/// Status of a link rule execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleStatus {
    /// When this rule was last executed
    pub last_run: Option<std::time::SystemTime>,
    /// Number of links created/modified on last run
    pub links_created: usize,
    /// Number of links that failed on last run
    pub links_failed: usize,
    /// Last error message, if any
    pub last_error: Option<String>,
    /// Total number of times this rule has run
    pub total_runs: usize,
}

impl Default for RuleStatus {
    fn default() -> Self {
        Self {
            last_run: None,
            links_created: 0,
            links_failed: 0,
            last_error: None,
            total_runs: 0,
        }
    }
}

/// Global application state (not tied to any specific node)
pub struct AppState {
    // Link rules to be monitored and relinked
    pub link_rules: Arc<Mutex<Vec<LinkRule>>>,
    // Status tracking for each rule (indexed by rule position)
    pub rule_status: Arc<Mutex<HashMap<usize, RuleStatus>>>,
    // Cache of PipeWire objects (id -> object)
    pub object_cache: Arc<RwLock<Vec<PwObject>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            link_rules: Arc::new(Mutex::new(Vec::new())),
            rule_status: Arc::new(Mutex::new(HashMap::new())),
            object_cache: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Load all PipeWire objects into the cache
    pub fn refresh_object_cache(&self) -> Result<(), String> {
        let objects = crate::pwcli::list_all()?;
        let count = objects.len();
        *self.object_cache.write().unwrap() = objects;
        info!("Loaded {} PipeWire objects into cache", count);
        Ok(())
    }

    /// Get all cached objects
    pub fn get_cached_objects(&self) -> Vec<PwObject> {
        self.object_cache.read().unwrap().clone()
    }

    /// Get a cached object by ID
    pub fn get_object_by_id(&self, id: u32) -> Option<PwObject> {
        self.object_cache.read().unwrap()
            .iter()
            .find(|o| o.id == id)
            .cloned()
    }

    /// Get objects by type
    pub fn get_objects_by_type(&self, obj_type: &str) -> Vec<PwObject> {
        self.object_cache.read().unwrap()
            .iter()
            .filter(|o| crate::pwcli::simplify_type(&o.object_type) == obj_type)
            .cloned()
            .collect()
    }

    /// Find object ID by name (searches node.name, device.name, etc.)
    pub fn find_id_by_name(&self, name: &str) -> Option<u32> {
        self.object_cache.read().unwrap()
            .iter()
            .find(|o| o.name().map(|n| n == name).unwrap_or(false))
            .map(|o| o.id)
    }

    /// Find object name by ID
    pub fn find_name_by_id(&self, id: u32) -> Option<String> {
        self.get_object_by_id(id)
            .and_then(|o| o.name().map(|s| s.to_string()))
    }

    pub fn set_link_rules(&self, rules: Vec<LinkRule>) {
        *self.link_rules.lock().unwrap() = rules;
    }

    pub fn get_link_rules(&self) -> Vec<LinkRule> {
        self.link_rules.lock().unwrap().clone()
    }

    /// Update the status of a rule after execution
    pub fn update_rule_status(&self, rule_idx: usize, links_created: usize, links_failed: usize, error: Option<String>) {
        let mut status_map = self.rule_status.lock().unwrap();
        let status = status_map.entry(rule_idx).or_insert_with(RuleStatus::default);
        
        status.last_run = Some(std::time::SystemTime::now());
        status.links_created = links_created;
        status.links_failed = links_failed;
        status.last_error = error;
        status.total_runs += 1;
    }

    /// Get the status of all rules
    pub fn get_all_rule_status(&self) -> HashMap<usize, RuleStatus> {
        self.rule_status.lock().unwrap().clone()
    }

    /// Get the status of a specific rule
    pub fn get_rule_status(&self, rule_idx: usize) -> Option<RuleStatus> {
        self.rule_status.lock().unwrap().get(&rule_idx).cloned()
    }
}

/// Node-specific state for modules that manage a specific PipeWire node
/// (e.g., speakereq, riaa)
pub struct NodeState {
    pub node_name: String,
    // Cache for parameters to avoid too many PipeWire calls
    // This is especially important for EQ parameters as external tools rarely change them
    pub cache: Arc<Mutex<Option<HashMap<String, ParameterValue>>>>,
}

impl NodeState {
    pub fn new(node_name: String) -> Self {
        Self {
            node_name,
            cache: Arc::new(Mutex::new(None)),
        }
    }

    // Get parameters using pw-cli (with caching to avoid excessive calls)
    // EQ parameters are cached as external tools rarely modify them
    pub fn get_params(&self) -> Result<HashMap<String, ParameterValue>, ApiError> {
        use crate::PipeWireClient;

        // Check cache first
        if let Some(ref cached) = *self.cache.lock().unwrap() {
            return Ok(cached.clone());
        }

        // Cache miss - fetch from PipeWire
        let client = PipeWireClient::new()
            .map_err(|e| ApiError::Internal(format!("Failed to connect to PipeWire: {}", e)))?;
        let (info, _node) = client.find_and_bind_node(&self.node_name, 2)
            .map_err(|e| ApiError::Internal(format!("Failed to find node: {}", e)))?;
        
        // Use pw-cli to enumerate parameters
        let params = Self::get_params_via_pwcli(info.id)
            .map_err(|e| ApiError::Internal(format!("Failed to get parameters: {}", e)))?;
        
        // Update cache
        *self.cache.lock().unwrap() = Some(params.clone());
        
        Ok(params)
    }

    /// Force refresh of parameter cache (use if external tools modified parameters)
    pub fn refresh_params_cache(&self) -> Result<(), ApiError> {
        *self.cache.lock().unwrap() = None;
        self.get_params()?; // Reload cache
        Ok(())
    }

    // Helper to set a single parameter using pw-cli
    pub fn set_parameter(&self, key: &str, value: ParameterValue) -> Result<(), ApiError> {
        let mut params = HashMap::new();
        params.insert(key.to_string(), value);
        self.set_parameters(params)
    }

    // Helper to set multiple parameters using pw-cli (batched in single call)
    pub fn set_parameters(&self, params: HashMap<String, ParameterValue>) -> Result<(), ApiError> {
        use crate::PipeWireClient;

        // Find the node ID
        let client = PipeWireClient::new()
            .map_err(|e| ApiError::Internal(format!("Failed to connect to PipeWire: {}", e)))?;
        let (info, _node) = client.find_and_bind_node(&self.node_name, 2)
            .map_err(|e| ApiError::Internal(format!("Failed to find node: {}", e)))?;
        
        // Build the JSON for pw-cli set-param
        Self::set_params_via_pwcli(info.id, params)
            .map_err(|e| ApiError::Internal(format!("Failed to set parameters: {}", e)))?;
        
        // Invalidate cache
        *self.cache.lock().unwrap() = None;
        
        Ok(())
    }

    // Parse pw-cli enum-params output to extract parameters
    fn get_params_via_pwcli(node_id: u32) -> Result<HashMap<String, ParameterValue>, String> {
        use std::process::Command;
        
        let output = Command::new("pw-cli")
            .args(["enum-params", &node_id.to_string(), "Props"])
            .output()
            .map_err(|e| format!("Failed to run pw-cli: {}", e))?;
        
        if !output.status.success() {
            return Err(format!("pw-cli failed: {}", String::from_utf8_lossy(&output.stderr)));
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        Self::parse_pw_cli_params(&stdout)
    }

    // Parse pw-cli output format
    fn parse_pw_cli_params(output: &str) -> Result<HashMap<String, ParameterValue>, String> {
        let mut params = HashMap::new();
        let lines: Vec<&str> = output.lines().collect();
        let mut i = 0;
        
        while i < lines.len() {
            let line = lines[i].trim();
            
            // Look for: String "speakereq2x2:parameter_name" or String "parameter_name"
            if line.starts_with("String ") {
                if let Some(key) = Self::extract_string_value(line) {
                    // Next line should have the value
                    if i + 1 < lines.len() {
                        let value_line = lines[i + 1].trim();
                        if let Some(value) = Self::parse_param_value(value_line) {
                            params.insert(key, value);
                        }
                    }
                }
            }
            i += 1;
        }
        
        Ok(params)
    }

    // Extract string value from: String "value"
    fn extract_string_value(line: &str) -> Option<String> {
        let start = line.find('"')?;
        let end = line.rfind('"')?;
        if start < end {
            Some(line[start + 1..end].to_string())
        } else {
            None
        }
    }

    // Parse parameter value from pw-cli output line
    fn parse_param_value(line: &str) -> Option<ParameterValue> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            match parts[0] {
                "Bool" => Some(ParameterValue::Bool(parts[1] == "true")),
                "Int" => parts[1].parse::<i32>().ok().map(ParameterValue::Int),
                "Float" => parts[1].parse::<f32>().ok().map(ParameterValue::Float),
                "String" => Self::extract_string_value(line).map(ParameterValue::String),
                _ => None,
            }
        } else {
            None
        }
    }

    // Set parameters using pw-cli
    fn set_params_via_pwcli(node_id: u32, params: HashMap<String, ParameterValue>) -> Result<(), String> {
        use std::process::Command;
        
        // Build array format for params struct: ["key1", value1, "key2", value2, ...]
        // This is the correct format for the SPA Struct in the params property
        let mut params_array = Vec::new();
        
        for (key, value) in params {
            params_array.push(serde_json::Value::String(key));
            
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
        let json = serde_json::json!({
            "params": params_array
        });
        let json_str = json.to_string();
        
        let output = Command::new("pw-cli")
            .args(["set-param", &node_id.to_string(), "Props", &json_str])
            .output()
            .map_err(|e| format!("Failed to run pw-cli: {}", e))?;
        
        if !output.status.success() {
            return Err(format!("pw-cli set-param failed: {}", String::from_utf8_lossy(&output.stderr)));
        }
        
        Ok(())
    }
}

// API error type
#[derive(Debug)]
pub enum ApiError {
    NotFound(String),
    BadRequest(String),
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };
        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}
