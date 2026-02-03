use axum::{
    extract::{Path, State},
    routing::{get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::collections::HashMap;
use crate::api_server::{ApiError, NodeState};
use crate::parameters::ParameterValue;

// EQ type constants
const EQ_TYPE_OFF: i32 = 0;
const EQ_TYPE_LOW_SHELF: i32 = 1;
const EQ_TYPE_HIGH_SHELF: i32 = 2;
const EQ_TYPE_PEAKING: i32 = 3;
const EQ_TYPE_LOW_PASS: i32 = 4;
const EQ_TYPE_HIGH_PASS: i32 = 5;
const EQ_TYPE_BAND_PASS: i32 = 6;
const EQ_TYPE_NOTCH: i32 = 7;
const EQ_TYPE_ALL_PASS: i32 = 8;

/// Helper to get the actual plugin name prefix from parameters
/// Returns the prefix like "speakereq2x2" from parameter names like "speakereq2x2:Enable"
pub fn get_plugin_prefix(params: &HashMap<String, ParameterValue>) -> String {
    // Find any parameter key that contains a colon and extract the prefix
    for key in params.keys() {
        if let Some(colon_pos) = key.find(':') {
            let prefix = &key[..colon_pos];
            if prefix.starts_with("speakereq") {
                return prefix.to_string();
            }
        }
    }
    // Default fallback
    "speakereq2x2".to_string()
}

/// Helper to create a prefixed parameter key
fn pkey(prefix: &str, param: &str) -> String {
    format!("{}:{}", prefix, param)
}

/// Count EQ slots for a given block by probing parameters
fn count_eq_slots(params: &HashMap<String, ParameterValue>, prefix: &str, block: &str) -> u32 {
    let mut slots = 0u32;
    for band in 1..=100 {
        let key = pkey(prefix, &format!("{}_eq_{}_type", block, band));
        if params.contains_key(&key) {
            slots = band;
        } else {
            break;
        }
    }
    slots
}

// API Models
#[derive(Debug, Serialize, Deserialize)]
pub struct StructureResponse {
    pub name: String,
    pub version: String,
    pub blocks: Vec<Block>,
    pub inputs: u32,
    pub outputs: u32,
    pub enabled: bool,
    pub licensed: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Block {
    pub id: String,
    #[serde(rename = "type")]
    pub block_type: String,
    pub slots: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IoResponse {
    pub inputs: u32,
    pub outputs: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EqBand {
    #[serde(rename = "type")]
    pub eq_type: String,
    pub frequency: f32,
    pub q: f32,
    pub gain: f32,
    #[serde(default = "default_enabled")]
    pub enabled: Option<bool>,
}

fn default_enabled() -> Option<bool> {
    Some(true)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GainValue {
    pub gain: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EnableValue {
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DelayValue {
    pub delay_ms: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlockStatus {
    pub id: String,
    #[serde(rename = "type")]
    pub block_type: String,
    pub gain_db: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay_ms: Option<f32>,
    pub eq_bands: Vec<EqBandStatus>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EqBandStatus {
    pub band: u32,
    #[serde(rename = "type")]
    pub eq_type: String,
    pub frequency: f32,
    pub q: f32,
    pub gain: f32,
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CrossbarMatrix {
    pub input_0_to_output_0: f32,
    pub input_0_to_output_1: f32,
    pub input_1_to_output_0: f32,
    pub input_1_to_output_1: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatusResponse {
    pub enabled: bool,
    pub master_gain_db: f32,
    pub crossbar: CrossbarMatrix,
    pub inputs: Vec<BlockStatus>,
    pub outputs: Vec<BlockStatus>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CrossbarMatrixResponse {
    pub matrix: Vec<Vec<f32>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CrossbarValueRequest {
    pub value: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CrossbarValueResponse {
    pub success: bool,
    pub input: usize,
    pub output: usize,
    pub value: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SetCrossbarMatrixRequest {
    pub matrix: Vec<Vec<f32>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SetCrossbarMatrixResponse {
    pub success: bool,
    pub matrix: Vec<Vec<f32>>,
}

// EQ type mapping
fn eq_type_to_string(type_id: i32) -> String {
    match type_id {
        EQ_TYPE_OFF => "off".to_string(),
        EQ_TYPE_LOW_SHELF => "low_shelf".to_string(),
        EQ_TYPE_HIGH_SHELF => "high_shelf".to_string(),
        EQ_TYPE_PEAKING => "peaking".to_string(),
        EQ_TYPE_LOW_PASS => "low_pass".to_string(),
        EQ_TYPE_HIGH_PASS => "high_pass".to_string(),
        EQ_TYPE_BAND_PASS => "band_pass".to_string(),
        EQ_TYPE_NOTCH => "notch".to_string(),
        EQ_TYPE_ALL_PASS => "all_pass".to_string(),
        _ => format!("unknown_{}", type_id),
    }
}

pub fn eq_type_from_string(type_str: &str) -> Result<i32, ApiError> {
    match type_str.to_lowercase().as_str() {
        "off" => Ok(EQ_TYPE_OFF),
        "low_shelf" => Ok(EQ_TYPE_LOW_SHELF),
        "high_shelf" => Ok(EQ_TYPE_HIGH_SHELF),
        "peaking" => Ok(EQ_TYPE_PEAKING),
        "low_pass" => Ok(EQ_TYPE_LOW_PASS),
        "high_pass" => Ok(EQ_TYPE_HIGH_PASS),
        "band_pass" => Ok(EQ_TYPE_BAND_PASS),
        "notch" => Ok(EQ_TYPE_NOTCH),
        "all_pass" => Ok(EQ_TYPE_ALL_PASS),
        _ => Err(ApiError::BadRequest(format!("Invalid EQ type: {}", type_str))),
    }
}

// Handlers
pub async fn get_structure(State(state): State<Arc<NodeState>>) -> Result<Json<StructureResponse>, ApiError> {
    let params = state.get_params()?;
    let prefix = get_plugin_prefix(&params);
    
    let enabled = params.get(&pkey(&prefix, "Enable"))
        .and_then(|v| match v {
            ParameterValue::Bool(b) => Some(*b),
            _ => None,
        })
        .unwrap_or(false);
    
    let licensed = params.get(&pkey(&prefix, "Licensed"))
        .and_then(|v| match v {
            ParameterValue::Bool(b) => Some(*b),
            _ => None,
        })
        .unwrap_or(true);
    
    Ok(Json(StructureResponse {
        name: prefix.clone(),
        version: "1.0".to_string(),
        blocks: vec![
            Block { id: "input_0".to_string(), block_type: "eq".to_string(), slots: 20 },
            Block { id: "input_1".to_string(), block_type: "eq".to_string(), slots: 20 },
            Block { id: "crossbar".to_string(), block_type: "crossbar".to_string(), slots: 4 },
            Block { id: "output_0".to_string(), block_type: "eq".to_string(), slots: 20 },
            Block { id: "output_1".to_string(), block_type: "eq".to_string(), slots: 20 },
            Block { id: "input_gain".to_string(), block_type: "volume".to_string(), slots: 2 },
            Block { id: "output_gain".to_string(), block_type: "volume".to_string(), slots: 2 },
            Block { id: "master_gain".to_string(), block_type: "volume".to_string(), slots: 1 },
        ],
        inputs: 2,
        outputs: 2,
        enabled,
        licensed,
    }))
}

pub async fn get_io() -> Json<IoResponse> {
    Json(IoResponse {
        inputs: 2,
        outputs: 2,
    })
}

/// Get plugin configuration by probing available parameters from PipeWire
pub async fn get_config(State(state): State<Arc<NodeState>>) -> Result<Json<serde_json::Value>, ApiError> {
    tracing::debug!("speakereq::get_config: starting");
    
    // Force refresh to ensure we have all parameters
    state.refresh_params_cache()?;
    let params = state.get_params()?;
    
    tracing::debug!("speakereq::get_config: got {} params", params.len());
    
    let prefix = get_plugin_prefix(&params);
    tracing::debug!("speakereq::get_config: prefix='{}', sample params: {:?}", 
        prefix, params.keys().take(5).collect::<Vec<_>>());
    
    // Probe for number of inputs/outputs by checking crossbar parameters
    // Crossbar uses xbar_{input}_to_{output} format
    let mut inputs = 0u32;
    let mut outputs = 0u32;
    
    // Count inputs by checking xbar_N_to_0 parameters
    for i in 0..16 {
        let key = pkey(&prefix, &format!("xbar_{}_to_0", i));
        if params.contains_key(&key) {
            inputs = i + 1;
        } else {
            break;
        }
    }
    
    // Count outputs by checking xbar_0_to_N parameters  
    for j in 0..16 {
        let key = pkey(&prefix, &format!("xbar_0_to_{}", j));
        if params.contains_key(&key) {
            outputs = j + 1;
        } else {
            break;
        }
    }
    
    tracing::debug!("speakereq::get_config: detected inputs={}, outputs={}", inputs, outputs);
    
    // Probe for number of EQ slots per block using shared helper
    let mut eq_slots = std::collections::HashMap::new();
    
    // Discover EQ blocks dynamically
    for i in 0..inputs {
        let block = format!("input_{}", i);
        let slots = count_eq_slots(&params, &prefix, &block);
        if slots > 0 {
            eq_slots.insert(block, slots);
        }
    }
    
    for j in 0..outputs {
        let block = format!("output_{}", j);
        let slots = count_eq_slots(&params, &prefix, &block);
        if slots > 0 {
            eq_slots.insert(block, slots);
        }
    }
    
    Ok(Json(serde_json::json!({
        "inputs": inputs,
        "outputs": outputs,
        "eq_slots": eq_slots,
        "plugin_name": prefix,
        "method": "probed_from_parameters"
    })))
}

pub async fn get_eq_band(
    State(state): State<Arc<NodeState>>,
    Path((block, band)): Path<(String, u32)>,
) -> Result<Json<EqBand>, ApiError> {
    let params = state.get_params()?;
    let prefix = get_plugin_prefix(&params);
    
    let type_key = pkey(&prefix, &format!("{}_eq_{}_type", block, band));
    let freq_key = pkey(&prefix, &format!("{}_eq_{}_f", block, band));
    let q_key = pkey(&prefix, &format!("{}_eq_{}_q", block, band));
    let gain_key = pkey(&prefix, &format!("{}_eq_{}_gain", block, band));
    let enabled_key = pkey(&prefix, &format!("{}_eq_{}_enabled", block, band));
    
    let eq_type = params.get(&type_key)
        .and_then(|v| match v {
            ParameterValue::Int(i) => Some(*i),
            _ => None,
        })
        .ok_or_else(|| ApiError::NotFound(format!("EQ band {}/{} not found", block, band)))?;
    
    let frequency = params.get(&freq_key)
        .and_then(|v| match v {
            ParameterValue::Float(f) => Some(*f),
            _ => None,
        })
        .unwrap_or(1000.0);
    
    let q = params.get(&q_key)
        .and_then(|v| match v {
            ParameterValue::Float(f) => Some(*f),
            ParameterValue::Int(i) => Some(*i as f32),
            _ => None,
        })
        .unwrap_or(1.0);
    
    let gain = params.get(&gain_key)
        .and_then(|v| match v {
            ParameterValue::Float(f) => Some(*f),
            ParameterValue::Int(i) => Some(*i as f32),
            _ => None,
        })
        .unwrap_or(0.0);
    
    let enabled = params.get(&enabled_key)
        .and_then(|v| match v {
            ParameterValue::Bool(b) => Some(*b),
            ParameterValue::Float(f) => Some(*f > 0.5),
            ParameterValue::Int(i) => Some(*i != 0),
            _ => None,
        })
        .unwrap_or(true);
    
    Ok(Json(EqBand {
        eq_type: eq_type_to_string(eq_type),
        frequency,
        q,
        gain,
        enabled: Some(enabled),
    }))
}

pub async fn set_eq_band(
    State(state): State<Arc<NodeState>>,
    Path((block, band)): Path<(String, u32)>,
    Json(eq_band): Json<EqBand>,
) -> Result<Json<EqBand>, ApiError> {
    // Validate input ranges
    if eq_band.frequency < 20.0 || eq_band.frequency > 20000.0 {
        return Err(ApiError::BadRequest("Frequency must be between 20 and 20000 Hz".to_string()));
    }
    if eq_band.q < 0.1 || eq_band.q > 10.0 {
        return Err(ApiError::BadRequest("Q must be between 0.1 and 10.0".to_string()));
    }
    if eq_band.gain < -24.0 || eq_band.gain > 24.0 {
        return Err(ApiError::BadRequest("Gain must be between -24 and +24 dB".to_string()));
    }
    
    let type_id = eq_type_from_string(&eq_band.eq_type)?;
    
    // Get prefix from existing params
    let existing_params = state.get_params()?;
    let prefix = get_plugin_prefix(&existing_params);
    
    // Build parameter keys with dynamic prefix
    let type_key = pkey(&prefix, &format!("{}_eq_{}_type", block, band));
    let freq_key = pkey(&prefix, &format!("{}_eq_{}_f", block, band));
    let q_key = pkey(&prefix, &format!("{}_eq_{}_q", block, band));
    let gain_key = pkey(&prefix, &format!("{}_eq_{}_gain", block, band));
    let enabled_key = pkey(&prefix, &format!("{}_eq_{}_enabled", block, band));
    
    // Batch all parameters into a single pw-cli call
    let mut params = std::collections::HashMap::new();
    params.insert(type_key, crate::parameters::ParameterValue::Int(type_id));
    params.insert(freq_key, crate::parameters::ParameterValue::Float(eq_band.frequency));
    params.insert(q_key, crate::parameters::ParameterValue::Float(eq_band.q));
    params.insert(gain_key, crate::parameters::ParameterValue::Float(eq_band.gain));
    
    // Set enabled parameter, default to true if not provided
    let enabled = eq_band.enabled.unwrap_or(true);
    params.insert(enabled_key, crate::parameters::ParameterValue::Bool(enabled));
    
    // Set all parameters at once
    state.set_parameters(params)?;
    
    Ok(Json(eq_band))
}

pub async fn clear_eq_bank(
    State(state): State<Arc<NodeState>>,
    Path(block): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Get prefix from existing params
    let existing_params = state.get_params()?;
    let prefix = get_plugin_prefix(&existing_params);
    
    // Dynamically count EQ slots for this block
    let slots = count_eq_slots(&existing_params, &prefix, &block);
    if slots == 0 {
        return Err(ApiError::NotFound(format!("No EQ bands found for block {}", block)));
    }
    
    // Clear all EQ bands by setting them to "off" (type = 0) in a single call
    let mut params = std::collections::HashMap::new();
    for band in 1..=slots {
        let type_key = pkey(&prefix, &format!("{}_eq_{}_type", block, band));
        params.insert(type_key, crate::parameters::ParameterValue::Int(0));
    }
    
    state.set_parameters(params)?;
    
    Ok(Json(serde_json::json!({
        "block": block,
        "slots_cleared": slots,
        "message": format!("All {} EQ bands cleared", slots)
    })))
}

pub async fn get_master_gain(State(state): State<Arc<NodeState>>) -> Result<Json<GainValue>, ApiError> {
    let params = state.get_params()?;
    let prefix = get_plugin_prefix(&params);
    
    let gain = params.get(&pkey(&prefix, "master_gain_db"))
        .and_then(|v| match v {
            ParameterValue::Float(f) => Some(*f),
            ParameterValue::Int(i) => Some(*i as f32),
            _ => None,
        })
        .ok_or_else(|| ApiError::NotFound("Master gain not found".to_string()))?;
    
    Ok(Json(GainValue { gain }))
}

pub async fn set_master_gain(
    State(state): State<Arc<NodeState>>,
    Json(gain_value): Json<GainValue>,
) -> Result<Json<GainValue>, ApiError> {
    if gain_value.gain < -60.0 || gain_value.gain > 12.0 {
        return Err(ApiError::BadRequest("Master gain must be between -60 and +12 dB".to_string()));
    }
    
    state.set_parameter("master_gain_db", ParameterValue::Float(gain_value.gain))?;
    
    Ok(Json(gain_value))
}

pub async fn get_enable(State(state): State<Arc<NodeState>>) -> Result<Json<EnableValue>, ApiError> {
    let params = state.get_params()?;
    let prefix = get_plugin_prefix(&params);
    
    let enabled = params.get(&pkey(&prefix, "Enable"))
        .and_then(|v| match v {
            ParameterValue::Bool(b) => Some(*b),
            _ => None,
        })
        .ok_or_else(|| ApiError::NotFound("Enable parameter not found".to_string()))?;
    
    Ok(Json(EnableValue { enabled }))
}

pub async fn set_enable(
    State(state): State<Arc<NodeState>>,
    Json(enable_value): Json<EnableValue>,
) -> Result<Json<EnableValue>, ApiError> {
    state.set_parameter("Enable", ParameterValue::Bool(enable_value.enabled))?;
    
    Ok(Json(enable_value))
}

pub async fn set_eq_band_enabled(
    State(state): State<Arc<NodeState>>,
    Path((block, band)): Path<(String, u32)>,
    Json(enable_value): Json<EnableValue>,
) -> Result<Json<EnableValue>, ApiError> {
    let params = state.get_params()?;
    let prefix = get_plugin_prefix(&params);
    
    let enabled_key = pkey(&prefix, &format!("{}_eq_{}_enabled", block, band));
    state.set_parameter(&enabled_key, ParameterValue::Bool(enable_value.enabled))?;
    
    Ok(Json(enable_value))
}

pub async fn get_status(State(state): State<Arc<NodeState>>) -> Result<Json<StatusResponse>, ApiError> {
    let params = state.get_params()?;
    let prefix = get_plugin_prefix(&params);
    
    // Get enable status
    let enabled = params.get(&pkey(&prefix, "Enable"))
        .and_then(|v| match v {
            ParameterValue::Bool(b) => Some(*b),
            _ => None,
        })
        .unwrap_or(false);
    
    // Get master gain
    let master_gain_db = params.get(&pkey(&prefix, "master_gain_db"))
        .and_then(|v| match v {
            ParameterValue::Float(f) => Some(*f),
            _ => None,
        })
        .unwrap_or(0.0);
    
    // Get crossbar matrix
    let xbar_0_to_0 = params.get(&pkey(&prefix, "xbar_0_to_0"))
        .and_then(|v| match v {
            ParameterValue::Float(f) => Some(*f),
            ParameterValue::Int(i) => Some(*i as f32),
            _ => None,
        })
        .unwrap_or(1.0);
    
    let xbar_0_to_1 = params.get(&pkey(&prefix, "xbar_0_to_1"))
        .and_then(|v| match v {
            ParameterValue::Float(f) => Some(*f),
            ParameterValue::Int(i) => Some(*i as f32),
            _ => None,
        })
        .unwrap_or(0.0);
    
    let xbar_1_to_0 = params.get(&pkey(&prefix, "xbar_1_to_0"))
        .and_then(|v| match v {
            ParameterValue::Float(f) => Some(*f),
            ParameterValue::Int(i) => Some(*i as f32),
            _ => None,
        })
        .unwrap_or(0.0);
    
    let xbar_1_to_1 = params.get(&pkey(&prefix, "xbar_1_to_1"))
        .and_then(|v| match v {
            ParameterValue::Float(f) => Some(*f),
            ParameterValue::Int(i) => Some(*i as f32),
            _ => None,
        })
        .unwrap_or(1.0);
    
    let crossbar = CrossbarMatrix {
        input_0_to_output_0: xbar_0_to_0,
        input_0_to_output_1: xbar_0_to_1,
        input_1_to_output_0: xbar_1_to_0,
        input_1_to_output_1: xbar_1_to_1,
    };
    
    // Helper function to get block status - capture prefix
    let get_block_status = |block_id: &str, block_type: &str, has_delay: bool, prefix: &str| -> Result<BlockStatus, ApiError> {
        // Get gain
        let gain_key = pkey(prefix, &format!("{}_gain_db", block_id));
        let gain_db = params.get(&gain_key)
            .and_then(|v| match v {
                ParameterValue::Float(f) => Some(*f),
                _ => None,
            })
            .unwrap_or(0.0);
        
        // Get delay (only for output blocks)
        let delay_ms = if has_delay {
            let delay_key = pkey(prefix, &format!("delay_{}_ms", block_id.split('_').last().unwrap_or("0")));
            params.get(&delay_key)
                .and_then(|v| match v {
                    ParameterValue::Float(f) => Some(*f),
                    ParameterValue::Int(i) => Some(*i as f32),
                    _ => None,
                })
        } else {
            None
        };
        
        // Get all EQ bands (assuming 20 bands)
        let mut eq_bands = Vec::new();
        for band in 1..=20 {
            let type_key = pkey(prefix, &format!("{}_eq_{}_type", block_id, band));
            let freq_key = pkey(prefix, &format!("{}_eq_{}_f", block_id, band));
            let q_key = pkey(prefix, &format!("{}_eq_{}_q", block_id, band));
            let gain_key = pkey(prefix, &format!("{}_eq_{}_gain", block_id, band));
            
            let eq_type_id = params.get(&type_key)
                .and_then(|v| match v {
                    ParameterValue::Int(i) => Some(*i),
                    _ => None,
                })
                .unwrap_or(0);
            
            let frequency = params.get(&freq_key)
                .and_then(|v| match v {
                    ParameterValue::Float(f) => Some(*f),
                    _ => None,
                })
                .unwrap_or(1000.0);
            
            let q = params.get(&q_key)
                .and_then(|v| match v {
                    ParameterValue::Float(f) => Some(*f),
                    _ => None,
                })
                .unwrap_or(1.0);
            
            let gain = params.get(&gain_key)
                .and_then(|v| match v {
                    ParameterValue::Float(f) => Some(*f),
                    _ => None,
                })
                .unwrap_or(0.0);
            
            let enabled_key = pkey(prefix, &format!("{}_eq_{}_enabled", block_id, band));
            let band_enabled = params.get(&enabled_key)
                .and_then(|v| match v {
                    ParameterValue::Bool(b) => Some(*b),
                    ParameterValue::Float(f) => Some(*f > 0.5),
                    ParameterValue::Int(i) => Some(*i != 0),
                    _ => None,
                })
                .unwrap_or(true);
            
            eq_bands.push(EqBandStatus {
                band,
                eq_type: eq_type_to_string(eq_type_id),
                frequency,
                q,
                gain,
                enabled: band_enabled,
            });
        }
        
        Ok(BlockStatus {
            id: block_id.to_string(),
            block_type: block_type.to_string(),
            gain_db,
            delay_ms,
            eq_bands,
        })
    };
    
    // Get all blocks
    let inputs = vec![
        get_block_status("input_0", "input", false, &prefix)?,
        get_block_status("input_1", "input", false, &prefix)?,
    ];
    
    let outputs = vec![
        get_block_status("output_0", "output", true, &prefix)?,
        get_block_status("output_1", "output", true, &prefix)?,
    ];
    
    Ok(Json(StatusResponse {
        enabled,
        master_gain_db,
        crossbar,
        inputs,
        outputs,
    }))
}

/// Get crossbar matrix in 2D array format
pub async fn get_crossbar(
    State(state): State<Arc<NodeState>>,
) -> Result<Json<CrossbarMatrixResponse>, ApiError> {
    let params = state.get_params()?;
    let prefix = get_plugin_prefix(&params);
    
    // Read all crossbar values
    let xbar_0_to_0 = params.get(&pkey(&prefix, "xbar_0_to_0"))
        .and_then(|v| match v {
            ParameterValue::Float(f) => Some(*f),
            ParameterValue::Int(i) => Some(*i as f32),
            _ => None,
        })
        .unwrap_or(1.0);
    
    let xbar_0_to_1 = params.get(&pkey(&prefix, "xbar_0_to_1"))
        .and_then(|v| match v {
            ParameterValue::Float(f) => Some(*f),
            ParameterValue::Int(i) => Some(*i as f32),
            _ => None,
        })
        .unwrap_or(0.0);
    
    let xbar_1_to_0 = params.get(&pkey(&prefix, "xbar_1_to_0"))
        .and_then(|v| match v {
            ParameterValue::Float(f) => Some(*f),
            ParameterValue::Int(i) => Some(*i as f32),
            _ => None,
        })
        .unwrap_or(0.0);
    
    let xbar_1_to_1 = params.get(&pkey(&prefix, "xbar_1_to_1"))
        .and_then(|v| match v {
            ParameterValue::Float(f) => Some(*f),
            ParameterValue::Int(i) => Some(*i as f32),
            _ => None,
        })
        .unwrap_or(1.0);
    
    // Format as 2D matrix: matrix[input][output]
    let matrix = vec![
        vec![xbar_0_to_0, xbar_0_to_1],
        vec![xbar_1_to_0, xbar_1_to_1],
    ];
    
    Ok(Json(CrossbarMatrixResponse { matrix }))
}

/// Set a single crossbar routing value
pub async fn set_crossbar_value(
    State(state): State<Arc<NodeState>>,
    Path((input, output)): Path<(usize, usize)>,
    Json(request): Json<CrossbarValueRequest>,
) -> Result<Json<CrossbarValueResponse>, ApiError> {
    // Validate indices
    if input > 1 || output > 1 {
        return Err(ApiError::BadRequest(
            "Input and output must be 0 or 1 for 2x2 crossbar".to_string()
        ));
    }
    
    // Validate value range
    if request.value < 0.0 || request.value > 2.0 {
        return Err(ApiError::BadRequest(
            "Crossbar value must be between 0.0 and 2.0".to_string()
        ));
    }
    
    // Set the parameter
    let param_name = format!("xbar_{}_to_{}", input, output);
    state.set_parameter(&param_name, ParameterValue::Float(request.value))?;
    
    Ok(Json(CrossbarValueResponse {
        success: true,
        input,
        output,
        value: request.value,
    }))
}

/// Set the entire crossbar matrix in one request
pub async fn set_crossbar_matrix(
    State(state): State<Arc<NodeState>>,
    Json(request): Json<SetCrossbarMatrixRequest>,
) -> Result<Json<SetCrossbarMatrixResponse>, ApiError> {
    // Validate matrix dimensions (must be 2x2)
    if request.matrix.len() != 2 {
        return Err(ApiError::BadRequest(
            "Crossbar matrix must have exactly 2 input rows".to_string()
        ));
    }
    
    for (i, row) in request.matrix.iter().enumerate() {
        if row.len() != 2 {
            return Err(ApiError::BadRequest(
                format!("Crossbar matrix row {} must have exactly 2 output columns", i)
            ));
        }
        
        // Validate value ranges
        for (j, &value) in row.iter().enumerate() {
            if value < 0.0 || value > 2.0 {
                return Err(ApiError::BadRequest(
                    format!("Crossbar value at [{},{}] = {} is out of range (0.0-2.0)", i, j, value)
                ));
            }
        }
    }
    
    // Set all crossbar parameters in one batch
    let mut params = std::collections::HashMap::new();
    params.insert("xbar_0_to_0".to_string(), ParameterValue::Float(request.matrix[0][0]));
    params.insert("xbar_0_to_1".to_string(), ParameterValue::Float(request.matrix[0][1]));
    params.insert("xbar_1_to_0".to_string(), ParameterValue::Float(request.matrix[1][0]));
    params.insert("xbar_1_to_1".to_string(), ParameterValue::Float(request.matrix[1][1]));
    
    state.set_parameters(params)?;
    
    Ok(Json(SetCrossbarMatrixResponse {
        success: true,
        matrix: request.matrix,
    }))
}

// Create router for speakereq endpoints
pub fn create_router(state: Arc<NodeState>) -> Router {
    Router::new()
        .route("/api/v1/module/speakereq/structure", get(get_structure))
        .route("/api/v1/module/speakereq/config", get(get_config))
        .route("/api/v1/module/speakereq/io", get(get_io))
        .route("/api/v1/module/speakereq/status", get(get_status))
        .route("/api/v1/module/speakereq/capabilities", get(get_capabilities))
        .route("/api/v1/module/speakereq/eq/:block/:band", get(get_eq_band).put(set_eq_band))
        .route("/api/v1/module/speakereq/eq/:block/:band/enabled", put(set_eq_band_enabled))
        .route("/api/v1/module/speakereq/eq/:block/clear", put(clear_eq_bank))
        .route("/api/v1/module/speakereq/gain/master", get(get_master_gain).put(set_master_gain))
        .route("/api/v1/module/speakereq/enable", get(get_enable).put(set_enable))
        .route("/api/v1/module/speakereq/crossbar", get(get_crossbar).put(set_crossbar_matrix))
        .route("/api/v1/module/speakereq/crossbar/:input/:output", put(set_crossbar_value))
        .route("/api/v1/module/speakereq/refresh", post(refresh_cache))
        .route("/api/v1/module/speakereq/default", post(set_default))
        .route("/api/v1/module/speakereq/save", post(save_config))
        .with_state(state)
}

/// Refresh parameter cache (use if external tools modified parameters)
pub async fn refresh_cache(
    State(state): State<Arc<NodeState>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state.refresh_params_cache()?;
    Ok(Json(serde_json::json!({
        "message": "Parameter cache refreshed"
    })))
}

/// Set all parameters to default values
pub async fn set_default(
    State(state): State<Arc<NodeState>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    use std::collections::HashMap;
    
    // Get structure to determine inputs, outputs, and EQ slots dynamically
    let structure = get_structure(State(state.clone())).await?;
    let structure_data = structure.0;
    
    let inputs = structure_data.inputs;
    let outputs = structure_data.outputs;
    
    // Find EQ blocks and their slot counts
    let mut eq_blocks = Vec::new();
    for block in &structure_data.blocks {
        if block.block_type == "eq" {
            eq_blocks.push((block.id.clone(), block.slots));
        }
    }
    
    let mut params = HashMap::new();
    
    // Set all gains to 0dB
    params.insert("master_gain_db".to_string(), ParameterValue::Float(0.0));
    for i in 0..inputs {
        params.insert(format!("input_{}_gain_db", i), ParameterValue::Float(0.0));
    }
    for i in 0..outputs {
        params.insert(format!("output_{}_gain_db", i), ParameterValue::Float(0.0));
    }
    
    // Set crossbar matrix to identity (1 on diagonal, 0 elsewhere)
    for i in 0..inputs {
        for j in 0..outputs {
            let value = if i == j { 1.0 } else { 0.0 };
            params.insert(format!("xbar_{}_to_{}", i, j), ParameterValue::Float(value));
        }
    }
    
    // Set all EQ bands to "off" for all EQ blocks
    for (block_id, slots) in eq_blocks {
        for band in 1..=slots {
            let type_key = format!("{}_eq_{}_type", block_id, band);
            let freq_key = format!("{}_eq_{}_f", block_id, band);
            let q_key = format!("{}_eq_{}_q", block_id, band);
            let gain_key = format!("{}_eq_{}_gain", block_id, band);
            let enabled_key = format!("{}_eq_{}_enabled", block_id, band);
            
            params.insert(type_key, ParameterValue::Int(EQ_TYPE_OFF));
            params.insert(freq_key, ParameterValue::Float(1000.0));
            params.insert(q_key, ParameterValue::Float(1.0));
            params.insert(gain_key, ParameterValue::Float(0.0));
            params.insert(enabled_key, ParameterValue::Bool(true));
        }
    }
    
    // Set enable to true
    params.insert("Enable".to_string(), ParameterValue::Bool(true));
    
    // Apply all parameters in batch
    state.set_parameters(params)?;
    
    Ok(Json(serde_json::json!({
        "status": "ok",
        "message": "All parameters set to default values"
    })))
}

/// Save current SpeakerEQ settings using the plugin's built-in save feature
/// This sets the "save_settings" control port to 1, which triggers the plugin
/// to save current parameters to ~/.config/ladspa/speakereq2x2.ini
pub async fn save_config(
    State(state): State<Arc<NodeState>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    use std::collections::HashMap;
    
    // Get the plugin prefix first
    let params = state.get_params()?;
    let prefix = get_plugin_prefix(&params);
    
    // Set "save_settings" to 1 to trigger the plugin to save
    let mut set_params = HashMap::new();
    set_params.insert(pkey(&prefix, "save_settings"), ParameterValue::Int(1));
    
    state.set_parameters(set_params)?;
    
    Ok(Json(serde_json::json!({
        "status": "ok",
        "message": "SpeakerEQ settings saved"
    })))
}

/// Response for capabilities endpoint
#[derive(Debug, Serialize, Deserialize)]
pub struct CapabilitiesResponse {
    pub eq_types: Vec<EqTypeInfo>,
    pub parameter_ranges: ParameterRanges,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EqTypeInfo {
    pub name: String,
    pub description: String,
    pub requires_gain: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ParameterRanges {
    pub frequency: RangeInfo,
    pub gain: RangeInfo,
    pub q: RangeInfo,
    pub crossbar: RangeInfo,
    pub master_gain: RangeInfo,
    pub delay: RangeInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RangeInfo {
    pub min: f32,
    pub max: f32,
    pub unit: String,
    pub description: String,
}

/// Get information about supported filter types and parameter ranges
pub async fn get_capabilities() -> Result<Json<CapabilitiesResponse>, ApiError> {
    let eq_types = vec![
        EqTypeInfo {
            name: "off".to_string(),
            description: "Disabled - no filtering".to_string(),
            requires_gain: false,
        },
        EqTypeInfo {
            name: "low_shelf".to_string(),
            description: "Low shelf filter - boosts/cuts low frequencies".to_string(),
            requires_gain: true,
        },
        EqTypeInfo {
            name: "high_shelf".to_string(),
            description: "High shelf filter - boosts/cuts high frequencies".to_string(),
            requires_gain: true,
        },
        EqTypeInfo {
            name: "peaking".to_string(),
            description: "Peaking filter - boosts/cuts around center frequency".to_string(),
            requires_gain: true,
        },
        EqTypeInfo {
            name: "low_pass".to_string(),
            description: "Low pass filter - passes frequencies below cutoff".to_string(),
            requires_gain: false,
        },
        EqTypeInfo {
            name: "high_pass".to_string(),
            description: "High pass filter - passes frequencies above cutoff".to_string(),
            requires_gain: false,
        },
        EqTypeInfo {
            name: "band_pass".to_string(),
            description: "Band pass filter - passes frequencies around center".to_string(),
            requires_gain: false,
        },
        EqTypeInfo {
            name: "notch".to_string(),
            description: "Notch filter - attenuates frequencies around center".to_string(),
            requires_gain: false,
        },
        EqTypeInfo {
            name: "all_pass".to_string(),
            description: "All pass filter - affects phase only".to_string(),
            requires_gain: false,
        },
    ];

    let parameter_ranges = ParameterRanges {
        frequency: RangeInfo {
            min: 20.0,
            max: 20000.0,
            unit: "Hz".to_string(),
            description: "Center/cutoff frequency for filters".to_string(),
        },
        gain: RangeInfo {
            min: -24.0,
            max: 24.0,
            unit: "dB".to_string(),
            description: "Gain adjustment for shelf/peaking filters".to_string(),
        },
        q: RangeInfo {
            min: 0.1,
            max: 10.0,
            unit: "Q factor".to_string(),
            description: "Filter quality factor (bandwidth control)".to_string(),
        },
        crossbar: RangeInfo {
            min: 0.0,
            max: 2.0,
            unit: "linear gain".to_string(),
            description: "Crossbar routing matrix values".to_string(),
        },
        master_gain: RangeInfo {
            min: -24.0,
            max: 24.0,
            unit: "dB".to_string(),
            description: "Master output gain".to_string(),
        },
        delay: RangeInfo {
            min: 0.0,
            max: 10.0,
            unit: "ms".to_string(),
            description: "Output delay compensation".to_string(),
        },
    };

    Ok(Json(CapabilitiesResponse {
        eq_types,
        parameter_ranges,
    }))
}
