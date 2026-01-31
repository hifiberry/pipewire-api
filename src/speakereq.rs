use axum::{
    extract::{Path, State},
    routing::{get, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::api_server::{ApiError, AppState};
use crate::parameters::ParameterValue;

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

// EQ type mapping
fn eq_type_to_string(type_id: i32) -> String {
    match type_id {
        0 => "off".to_string(),
        1 => "low_shelf".to_string(),
        2 => "high_shelf".to_string(),
        3 => "peaking".to_string(),
        4 => "low_pass".to_string(),
        5 => "high_pass".to_string(),
        6 => "band_pass".to_string(),
        7 => "notch".to_string(),
        8 => "all_pass".to_string(),
        _ => format!("unknown_{}", type_id),
    }
}

fn eq_type_from_string(type_str: &str) -> Result<i32, ApiError> {
    match type_str.to_lowercase().as_str() {
        "off" => Ok(0),
        "low_shelf" => Ok(1),
        "high_shelf" => Ok(2),
        "peaking" => Ok(3),
        "low_pass" => Ok(4),
        "high_pass" => Ok(5),
        "band_pass" => Ok(6),
        "notch" => Ok(7),
        "all_pass" => Ok(8),
        _ => Err(ApiError::BadRequest(format!("Invalid EQ type: {}", type_str))),
    }
}

// Handlers
pub async fn get_structure(State(state): State<Arc<AppState>>) -> Result<Json<StructureResponse>, ApiError> {
    let params = state.get_params()?;
    
    let enabled = params.get("speakereq2x2:Enable")
        .and_then(|v| match v {
            ParameterValue::Bool(b) => Some(*b),
            _ => None,
        })
        .unwrap_or(false);
    
    let licensed = params.get("speakereq2x2:Licensed")
        .and_then(|v| match v {
            ParameterValue::Bool(b) => Some(*b),
            _ => None,
        })
        .unwrap_or(true);
    
    Ok(Json(StructureResponse {
        name: "speakereq2x2".to_string(),
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

pub async fn get_eq_band(
    State(state): State<Arc<AppState>>,
    Path((block, band)): Path<(String, u32)>,
) -> Result<Json<EqBand>, ApiError> {
    let params = state.get_params()?;
    
    let type_key = format!("speakereq2x2:{}_eq_{}_type", block, band);
    let freq_key = format!("speakereq2x2:{}_eq_{}_f", block, band);
    let q_key = format!("speakereq2x2:{}_eq_{}_q", block, band);
    let gain_key = format!("speakereq2x2:{}_eq_{}_gain", block, band);
    let enabled_key = format!("speakereq2x2:{}_eq_{}_enabled", block, band);
    
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
    State(state): State<Arc<AppState>>,
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
    
    // Set all parameters
    let type_key = format!("{}_eq_{}_type", block, band);
    let freq_key = format!("{}_eq_{}_f", block, band);
    let q_key = format!("{}_eq_{}_q", block, band);
    let gain_key = format!("{}_eq_{}_gain", block, band);
    let enabled_key = format!("{}_eq_{}_enabled", block, band);
    
    state.set_parameter(&type_key, ParameterValue::Int(type_id))?;
    state.set_parameter(&freq_key, ParameterValue::Float(eq_band.frequency))?;
    state.set_parameter(&q_key, ParameterValue::Float(eq_band.q))?;
    state.set_parameter(&gain_key, ParameterValue::Float(eq_band.gain))?;
    
    // Set enabled parameter, default to true if not provided
    let enabled = eq_band.enabled.unwrap_or(true);
    state.set_parameter(&enabled_key, ParameterValue::Bool(enabled))?;
    
    Ok(Json(eq_band))
}

pub async fn clear_eq_bank(
    State(state): State<Arc<AppState>>,
    Path(block): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Clear all 20 EQ bands by setting them to "off" (type = 0)
    for band in 1..=20 {
        let type_key = format!("{}_eq_{}_type", block, band);
        state.set_parameter(&type_key, ParameterValue::Int(0))?;
    }
    
    Ok(Json(serde_json::json!({
        "block": block,
        "message": "All EQ bands cleared"
    })))
}

pub async fn get_master_gain(State(state): State<Arc<AppState>>) -> Result<Json<GainValue>, ApiError> {
    let params = state.get_params()?;
    
    let gain = params.get("speakereq2x2:master_gain_db")
        .and_then(|v| match v {
            ParameterValue::Float(f) => Some(*f),
            ParameterValue::Int(i) => Some(*i as f32),
            _ => None,
        })
        .ok_or_else(|| ApiError::NotFound("Master gain not found".to_string()))?;
    
    Ok(Json(GainValue { gain }))
}

pub async fn set_master_gain(
    State(state): State<Arc<AppState>>,
    Json(gain_value): Json<GainValue>,
) -> Result<Json<GainValue>, ApiError> {
    if gain_value.gain < -60.0 || gain_value.gain > 12.0 {
        return Err(ApiError::BadRequest("Master gain must be between -60 and +12 dB".to_string()));
    }
    
    state.set_parameter("master_gain_db", ParameterValue::Float(gain_value.gain))?;
    
    Ok(Json(gain_value))
}

pub async fn get_enable(State(state): State<Arc<AppState>>) -> Result<Json<EnableValue>, ApiError> {
    let params = state.get_params()?;
    
    let enabled = params.get("speakereq2x2:Enable")
        .and_then(|v| match v {
            ParameterValue::Bool(b) => Some(*b),
            _ => None,
        })
        .ok_or_else(|| ApiError::NotFound("Enable parameter not found".to_string()))?;
    
    Ok(Json(EnableValue { enabled }))
}

pub async fn set_enable(
    State(state): State<Arc<AppState>>,
    Json(enable_value): Json<EnableValue>,
) -> Result<Json<EnableValue>, ApiError> {
    state.set_parameter("Enable", ParameterValue::Bool(enable_value.enabled))?;
    
    Ok(Json(enable_value))
}

pub async fn set_eq_band_enabled(
    State(state): State<Arc<AppState>>,
    Path((block, band)): Path<(String, u32)>,
    Json(enable_value): Json<EnableValue>,
) -> Result<Json<EnableValue>, ApiError> {
    let enabled_key = format!("{}_eq_{}_enabled", block, band);
    state.set_parameter(&enabled_key, ParameterValue::Bool(enable_value.enabled))?;
    
    Ok(Json(enable_value))
}

pub async fn get_status(State(state): State<Arc<AppState>>) -> Result<Json<StatusResponse>, ApiError> {
    let params = state.get_params()?;
    
    // Get enable status
    let enabled = params.get("speakereq2x2:Enable")
        .and_then(|v| match v {
            ParameterValue::Bool(b) => Some(*b),
            _ => None,
        })
        .unwrap_or(false);
    
    // Get master gain
    let master_gain_db = params.get("speakereq2x2:master_gain_db")
        .and_then(|v| match v {
            ParameterValue::Float(f) => Some(*f),
            _ => None,
        })
        .unwrap_or(0.0);
    
    // Get crossbar matrix
    let xbar_0_to_0 = params.get("speakereq2x2:xbar_0_to_0")
        .and_then(|v| match v {
            ParameterValue::Float(f) => Some(*f),
            ParameterValue::Int(i) => Some(*i as f32),
            _ => None,
        })
        .unwrap_or(1.0);
    
    let xbar_0_to_1 = params.get("speakereq2x2:xbar_0_to_1")
        .and_then(|v| match v {
            ParameterValue::Float(f) => Some(*f),
            ParameterValue::Int(i) => Some(*i as f32),
            _ => None,
        })
        .unwrap_or(0.0);
    
    let xbar_1_to_0 = params.get("speakereq2x2:xbar_1_to_0")
        .and_then(|v| match v {
            ParameterValue::Float(f) => Some(*f),
            ParameterValue::Int(i) => Some(*i as f32),
            _ => None,
        })
        .unwrap_or(0.0);
    
    let xbar_1_to_1 = params.get("speakereq2x2:xbar_1_to_1")
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
    
    // Helper function to get block status
    let get_block_status = |block_id: &str, block_type: &str, has_delay: bool| -> Result<BlockStatus, ApiError> {
        // Get gain
        let gain_key = format!("speakereq2x2:{}_gain_db", block_id);
        let gain_db = params.get(&gain_key)
            .and_then(|v| match v {
                ParameterValue::Float(f) => Some(*f),
                _ => None,
            })
            .unwrap_or(0.0);
        
        // Get delay (only for output blocks)
        let delay_ms = if has_delay {
            let delay_key = format!("speakereq2x2:delay_{}_ms", block_id.split('_').last().unwrap_or("0"));
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
            let type_key = format!("speakereq2x2:{}_eq_{}_type", block_id, band);
            let freq_key = format!("speakereq2x2:{}_eq_{}_f", block_id, band);
            let q_key = format!("speakereq2x2:{}_eq_{}_q", block_id, band);
            let gain_key = format!("speakereq2x2:{}_eq_{}_gain", block_id, band);
            
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
            
            let enabled_key = format!("speakereq2x2:{}_eq_{}_enabled", block_id, band);
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
        get_block_status("input_0", "input", false)?,
        get_block_status("input_1", "input", false)?,
    ];
    
    let outputs = vec![
        get_block_status("output_0", "output", true)?,
        get_block_status("output_1", "output", true)?,
    ];
    
    Ok(Json(StatusResponse {
        enabled,
        master_gain_db,
        crossbar,
        inputs,
        outputs,
    }))
}

// Create router for speakereq endpoints
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/v1/speakereq/structure", get(get_structure))
        .route("/api/v1/speakereq/io", get(get_io))
        .route("/api/v1/speakereq/status", get(get_status))
        .route("/api/v1/speakereq/eq/:block/:band", get(get_eq_band).put(set_eq_band))
        .route("/api/v1/speakereq/eq/:block/:band/enabled", put(set_eq_band_enabled))
        .route("/api/v1/speakereq/eq/:block/clear", put(clear_eq_bank))
        .route("/api/v1/speakereq/gain/master", get(get_master_gain).put(set_master_gain))
        .route("/api/v1/speakereq/enable", get(get_enable).put(set_enable))
        .with_state(state)
}
