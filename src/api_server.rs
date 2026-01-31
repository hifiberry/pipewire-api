use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use crate::parameters::ParameterValue;
use crate::linker::LinkRule;
use serde::{Deserialize, Serialize};

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

// Since PipeWire Node is not Send/Sync, we store just the node name
// and recreate connections per request (or use a message passing pattern)
pub struct AppState {
    pub node_name: String,
    // Cache for parameters to avoid too many PipeWire calls
    pub cache: Arc<Mutex<Option<HashMap<String, ParameterValue>>>>,
    // Link rules to be monitored and relinked
    pub link_rules: Arc<Mutex<Vec<LinkRule>>>,
    // Status tracking for each rule (indexed by rule position)
    pub rule_status: Arc<Mutex<HashMap<usize, RuleStatus>>>,
}

impl AppState {
    pub fn new(node_name: String) -> Self {
        Self {
            node_name,
            cache: Arc::new(Mutex::new(None)),
            link_rules: Arc::new(Mutex::new(Vec::new())),
            rule_status: Arc::new(Mutex::new(HashMap::new())),
        }
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

    // Helper to get parameters (with caching)
    pub fn get_params(&self) -> Result<HashMap<String, ParameterValue>, ApiError> {
        use crate::PipeWireClient;
        use crate::parameters::get_all_params;

        let client = PipeWireClient::new()
            .map_err(|e| ApiError::Internal(format!("Failed to connect to PipeWire: {}", e)))?;
        
        let (_info, node) = client.find_and_bind_node(&self.node_name, 2)
            .map_err(|e| ApiError::Internal(format!("Failed to find node: {}", e)))?;
        
        let params = get_all_params(&node, client.mainloop())
            .map_err(|e| ApiError::Internal(format!("Failed to get parameters: {}", e)))?;
        
        // Update cache
        *self.cache.lock().unwrap() = Some(params.clone());
        
        Ok(params)
    }

    // Helper to set a parameter
    pub fn set_parameter(&self, key: &str, value: ParameterValue) -> Result<(), ApiError> {
        use crate::PipeWireClient;
        use crate::parameters::set_param;

        let client = PipeWireClient::new()
            .map_err(|e| ApiError::Internal(format!("Failed to connect to PipeWire: {}", e)))?;
        
        let (_info, node) = client.find_and_bind_node(&self.node_name, 2)
            .map_err(|e| ApiError::Internal(format!("Failed to find node: {}", e)))?;
        
        set_param(&node, client.mainloop(), key, value)
            .map_err(|e| ApiError::Internal(format!("Failed to set parameter: {}", e)))?;
        
        // Invalidate cache
        *self.cache.lock().unwrap() = None;
        
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
