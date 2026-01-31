use axum::{
    extract::State,
    Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, info};

use crate::api_server::{ApiError, AppState};
use crate::linker::LinkRule;
use crate::link_manager::apply_link_rule as apply_link_rule_internal;
use crate::pipewire_client::PipeWireClient;

/// Create the router for link management endpoints
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/v1/links", get(list_links))
        .route("/api/v1/links/apply", post(apply_link_rule))
        .route("/api/v1/links/batch", post(apply_batch_rules))
        .route("/api/v1/links/default", get(get_default_rules))
        .route("/api/v1/links/apply-defaults", post(apply_default_rules))
        .route("/api/v1/links/status", get(get_link_rules_status))
        .with_state(state)
}

/// Response for link operations
#[derive(Debug, Serialize)]
pub struct LinkResponse {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

/// Response for listing active links
#[derive(Debug, Clone, Serialize)]
pub struct LinkInfo {
    pub id: u32,
    pub output_node_id: u32,
    pub output_port_id: u32,
    pub input_node_id: u32,
    pub input_port_id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_node_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_node_name: Option<String>,
}

/// Apply a link rule
pub async fn apply_link_rule(
    State(_state): State<Arc<AppState>>,
    Json(rule): Json<LinkRule>,
) -> Result<Json<LinkResponse>, ApiError> {
    info!("Applying link rule: {:?}", rule);

    // Get PipeWire client
    let client = PipeWireClient::new()
        .map_err(|e| ApiError::Internal(format!("Failed to create PipeWire client: {}", e)))?;

    // Apply the rule
    match apply_link_rule_internal(client.registry(), client.core(), client.mainloop(), &rule) {
        Ok(results) => {
            let success = results.iter().all(|r| r.success);
            let messages: Vec<String> = results.iter().map(|r| r.message.clone()).collect();
            let message = messages.join("; ");
            
            info!("Link rule application complete: success={}", success);
            Ok(Json(LinkResponse {
                success,
                message: if message.is_empty() { "Link rule applied".to_string() } else { message },
                details: None,
            }))
        }
        Err(e) => {
            error!("Failed to apply link rule: {}", e);
            Err(ApiError::Internal(format!("Failed to apply link rule: {}", e)))
        }
    }
}

/// List all active PipeWire links
pub async fn list_links(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<Vec<LinkInfo>>, ApiError> {
    use pipewire as pw;
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::collections::HashMap;
    
    debug!("Listing all PipeWire links");

    let client = PipeWireClient::new()
        .map_err(|e| ApiError::Internal(format!("Failed to create PipeWire client: {}", e)))?;

    let link_infos: Rc<RefCell<Vec<LinkInfo>>> = Rc::new(RefCell::new(Vec::new()));
    let link_infos_clone = link_infos.clone();
    
    // Also collect node names for reference
    let node_names: Rc<RefCell<HashMap<u32, String>>> = Rc::new(RefCell::new(HashMap::new()));
    let node_names_clone = node_names.clone();
    
    // Set up timeout
    let timeout_mainloop = client.mainloop().clone();
    let _timer = client.mainloop().loop_().add_timer(move |_| {
        timeout_mainloop.quit();
    });
    _timer.update_timer(Some(std::time::Duration::from_secs(2)), None);
    
    let _listener = client.registry()
        .add_listener_local()
        .global({
            move |global| {
                if let Some(props) = &global.props {
                    // Collect node names
                    if global.type_ == pw::types::ObjectType::Node {
                        if let Some(name) = props.get("node.name") {
                            node_names_clone.borrow_mut().insert(global.id, name.to_string());
                        }
                    }
                    
                    // Collect links
                    if global.type_ == pw::types::ObjectType::Link {
                        let output_node_id = props.get("link.output.node")
                            .and_then(|s| s.parse::<u32>().ok())
                            .unwrap_or(0);
                        let output_port_id = props.get("link.output.port")
                            .and_then(|s| s.parse::<u32>().ok())
                            .unwrap_or(0);
                        let input_node_id = props.get("link.input.node")
                            .and_then(|s| s.parse::<u32>().ok())
                            .unwrap_or(0);
                        let input_port_id = props.get("link.input.port")
                            .and_then(|s| s.parse::<u32>().ok())
                            .unwrap_or(0);
                        
                        link_infos_clone.borrow_mut().push(LinkInfo {
                            id: global.id,
                            output_node_id,
                            output_port_id,
                            input_node_id,
                            input_port_id,
                            output_node_name: None, // Will fill in after
                            input_node_name: None,
                        });
                    }
                }
            }
        })
        .register();
    
    client.mainloop().run();
    
    // Now fill in node names
    let node_names_map = node_names.borrow();
    let mut links = link_infos.borrow_mut();
    for link in links.iter_mut() {
        link.output_node_name = node_names_map.get(&link.output_node_id).cloned();
        link.input_node_name = node_names_map.get(&link.input_node_id).cloned();
    }
    
    let result = links.clone();
    debug!("Found {} links", result.len());
    Ok(Json(result))
}

/// Request to apply multiple link rules
#[derive(Debug, Deserialize)]
pub struct BatchLinkRequest {
    pub rules: Vec<LinkRule>,
}

/// Response for batch operations
#[derive(Debug, Serialize)]
pub struct BatchLinkResponse {
    pub total: usize,
    pub successful: usize,
    pub failed: usize,
    pub results: Vec<LinkResponse>,
}

/// Apply multiple link rules in sequence
pub async fn apply_batch_rules(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<BatchLinkRequest>,
) -> Result<Json<BatchLinkResponse>, ApiError> {
    info!("Applying batch of {} link rules", request.rules.len());

    let client = PipeWireClient::new()
        .map_err(|e| ApiError::Internal(format!("Failed to create PipeWire client: {}", e)))?;

    let total = request.rules.len();
    let mut successful = 0;
    let mut failed = 0;
    let mut results = Vec::new();

    for (idx, rule) in request.rules.iter().enumerate() {
        debug!("Applying rule {}/{}", idx + 1, total);
        
        match apply_link_rule_internal(client.registry(), client.core(), client.mainloop(), rule) {
            Ok(link_results) => {
                let all_success = link_results.iter().all(|r| r.success);
                let messages: Vec<String> = link_results.iter().map(|r| r.message.clone()).collect();
                let message = messages.join("; ");
                
                if all_success {
                    successful += 1;
                } else {
                    failed += 1;
                }
                
                results.push(LinkResponse {
                    success: all_success,
                    message: if message.is_empty() {
                        format!("Rule {} applied", idx + 1)
                    } else {
                        format!("Rule {}: {}", idx + 1, message)
                    },
                    details: None,
                });
            }
            Err(e) => {
                failed += 1;
                error!("Failed to apply rule {}: {}", idx + 1, e);
                results.push(LinkResponse {
                    success: false,
                    message: format!("Rule {} failed: {}", idx + 1, e),
                    details: None,
                });
            }
        }
    }

    info!("Batch complete: {}/{} successful, {} failed", successful, total, failed);
    Ok(Json(BatchLinkResponse {
        total,
        successful,
        failed,
        results,
    }))
}

/// Get the default link rules
pub async fn get_default_rules(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<Vec<LinkRule>>, ApiError> {
    use crate::default_link_rules;
    
    debug!("Retrieving default link rules");
    let rules = default_link_rules::get_default_rules();
    Ok(Json(rules))
}

/// Apply the default link rules
pub async fn apply_default_rules(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<BatchLinkResponse>, ApiError> {
    use crate::default_link_rules;
    
    info!("Applying default link rules");
    
    let rules = default_link_rules::get_default_rules();
    let client = PipeWireClient::new()
        .map_err(|e| ApiError::Internal(format!("Failed to create PipeWire client: {}", e)))?;

    let total = rules.len();
    let mut successful = 0;
    let mut failed = 0;
    let mut results = Vec::new();

    for (idx, rule) in rules.iter().enumerate() {
        debug!("Applying default rule {}/{}", idx + 1, total);
        
        match apply_link_rule_internal(client.registry(), client.core(), client.mainloop(), rule) {
            Ok(link_results) => {
                let all_success = link_results.iter().all(|r| r.success);
                let messages: Vec<String> = link_results.iter().map(|r| r.message.clone()).collect();
                let message = messages.join("; ");
                
                if all_success {
                    successful += 1;
                } else {
                    failed += 1;
                }
                
                results.push(LinkResponse {
                    success: all_success,
                    message: if message.is_empty() {
                        format!("Default rule {} applied successfully", idx + 1)
                    } else {
                        format!("Default rule {}: {}", idx + 1, message)
                    },
                    details: None,
                });
            }
            Err(e) => {
                failed += 1;
                error!("Failed to apply default rule {}: {}", idx + 1, e);
                results.push(LinkResponse {
                    success: false,
                    message: format!("Default rule {} failed: {}", idx + 1, e),
                    details: None,
                });
            }
        }
    }

    info!("Default rules complete: {}/{} successful, {} failed", successful, total, failed);
    Ok(Json(BatchLinkResponse {
        total,
        successful,
        failed,
        results,
    }))
}

/// Response for link rule status
#[derive(Debug, Serialize)]
pub struct LinkRuleStatusResponse {
    pub rules: Vec<LinkRuleWithStatus>,
}

/// A link rule with its execution status
#[derive(Debug, Serialize)]
pub struct LinkRuleWithStatus {
    pub index: usize,
    pub rule: LinkRule,
    pub status: Option<RuleStatusInfo>,
}

/// Serializable version of RuleStatus with formatted timestamps
#[derive(Debug, Serialize)]
pub struct RuleStatusInfo {
    pub last_run: Option<String>,
    pub last_run_timestamp: Option<u64>,
    pub links_created: usize,
    pub links_failed: usize,
    pub last_error: Option<String>,
    pub total_runs: usize,
}

/// Get status of all link rules being monitored
pub async fn get_link_rules_status(
    State(state): State<Arc<AppState>>,
) -> Result<Json<LinkRuleStatusResponse>, ApiError> {
    debug!("Retrieving link rules status");
    
    let rules = state.get_link_rules();
    let all_status = state.get_all_rule_status();
    
    let rules_with_status: Vec<LinkRuleWithStatus> = rules
        .into_iter()
        .enumerate()
        .map(|(idx, rule)| {
            let status = all_status.get(&idx).map(|s| {
                let (last_run_str, last_run_ts) = if let Some(last_run) = s.last_run {
                    let duration = last_run
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or(std::time::Duration::from_secs(0));
                    let timestamp = duration.as_secs();
                    
                    // Format as ISO 8601
                    let datetime = humantime::format_rfc3339(last_run).to_string();
                    (Some(datetime), Some(timestamp))
                } else {
                    (None, None)
                };
                
                RuleStatusInfo {
                    last_run: last_run_str,
                    last_run_timestamp: last_run_ts,
                    links_created: s.links_created,
                    links_failed: s.links_failed,
                    last_error: s.last_error.clone(),
                    total_runs: s.total_runs,
                }
            });
            
            LinkRuleWithStatus {
                index: idx,
                rule,
                status,
            }
        })
        .collect();
    
    Ok(Json(LinkRuleStatusResponse {
        rules: rules_with_status,
    }))
}
