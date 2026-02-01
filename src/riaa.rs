use axum::{
    extract::State,
    routing::{get, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::api_server::{ApiError, NodeState};
use crate::parameters::ParameterValue;

// API Models
#[derive(Debug, Serialize, Deserialize)]
pub struct RiaaConfig {
    pub gain_db: f32,
    pub subsonic_filter: i32,
    pub riaa_enable: bool,
    pub declick_enable: bool,
    pub spike_threshold_db: f32,
    pub spike_width_ms: f32,
    pub notch_filter_enable: bool,
    pub notch_frequency_hz: f32,
    pub notch_q_factor: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GainValue {
    pub gain_db: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubsonicFilterValue {
    pub filter: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EnableValue {
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpikeConfig {
    pub threshold_db: f32,
    pub width_ms: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NotchConfig {
    pub enabled: bool,
    pub frequency_hz: f32,
    pub q_factor: f32,
}

// Handlers
pub async fn get_config(State(state): State<Arc<NodeState>>) -> Result<Json<RiaaConfig>, ApiError> {
    let params = state.get_params()?;
    
    let gain_db = params.get("riaa:Gain (dB)")
        .and_then(|v| match v {
            ParameterValue::Float(f) => Some(*f),
            _ => None,
        })
        .unwrap_or(0.0);
    
    let subsonic_filter = params.get("riaa:Subsonic Filter")
        .and_then(|v| match v {
            ParameterValue::Int(i) => Some(*i),
            _ => None,
        })
        .unwrap_or(0);
    
    let riaa_enable = params.get("riaa:RIAA Enable")
        .and_then(|v| match v {
            ParameterValue::Bool(b) => Some(*b),
            _ => None,
        })
        .unwrap_or(true);
    
    let declick_enable = params.get("riaa:Declick Enable")
        .and_then(|v| match v {
            ParameterValue::Bool(b) => Some(*b),
            _ => None,
        })
        .unwrap_or(false);
    
    let spike_threshold_db = params.get("riaa:Spike Threshold (dB)")
        .and_then(|v| match v {
            ParameterValue::Float(f) => Some(*f),
            _ => None,
        })
        .unwrap_or(20.0);
    
    let spike_width_ms = params.get("riaa:Spike Width (ms)")
        .and_then(|v| match v {
            ParameterValue::Float(f) => Some(*f),
            _ => None,
        })
        .unwrap_or(1.0);
    
    let notch_filter_enable = params.get("riaa:Notch Filter Enable")
        .and_then(|v| match v {
            ParameterValue::Bool(b) => Some(*b),
            _ => None,
        })
        .unwrap_or(false);
    
    let notch_frequency_hz = params.get("riaa:Notch Frequency (Hz)")
        .and_then(|v| match v {
            ParameterValue::Float(f) => Some(*f),
            _ => None,
        })
        .unwrap_or(250.0);
    
    let notch_q_factor = params.get("riaa:Notch Q Factor")
        .and_then(|v| match v {
            ParameterValue::Float(f) => Some(*f),
            _ => None,
        })
        .unwrap_or(25.0);
    
    Ok(Json(RiaaConfig {
        gain_db,
        subsonic_filter,
        riaa_enable,
        declick_enable,
        spike_threshold_db,
        spike_width_ms,
        notch_filter_enable,
        notch_frequency_hz,
        notch_q_factor,
    }))
}

pub async fn get_gain(State(state): State<Arc<NodeState>>) -> Result<Json<GainValue>, ApiError> {
    let params = state.get_params()?;
    
    let gain_db = params.get("riaa:Gain (dB)")
        .and_then(|v| match v {
            ParameterValue::Float(f) => Some(*f),
            _ => None,
        })
        .unwrap_or(0.0);
    
    Ok(Json(GainValue { gain_db }))
}

pub async fn set_gain(
    State(state): State<Arc<NodeState>>,
    Json(gain_value): Json<GainValue>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state.set_parameter("riaa:Gain (dB)", ParameterValue::Float(gain_value.gain_db))?;
    
    Ok(Json(serde_json::json!({
        "status": "ok",
        "gain_db": gain_value.gain_db
    })))
}

pub async fn get_subsonic_filter(State(state): State<Arc<NodeState>>) -> Result<Json<SubsonicFilterValue>, ApiError> {
    let params = state.get_params()?;
    
    let filter = params.get("riaa:Subsonic Filter")
        .and_then(|v| match v {
            ParameterValue::Int(i) => Some(*i),
            _ => None,
        })
        .unwrap_or(0);
    
    Ok(Json(SubsonicFilterValue { filter }))
}

pub async fn set_subsonic_filter(
    State(state): State<Arc<NodeState>>,
    Json(filter_value): Json<SubsonicFilterValue>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state.set_parameter("riaa:Subsonic Filter", ParameterValue::Int(filter_value.filter))?;
    
    Ok(Json(serde_json::json!({
        "status": "ok",
        "filter": filter_value.filter
    })))
}

pub async fn get_riaa_enable(State(state): State<Arc<NodeState>>) -> Result<Json<EnableValue>, ApiError> {
    let params = state.get_params()?;
    
    let enabled = params.get("riaa:RIAA Enable")
        .and_then(|v| match v {
            ParameterValue::Bool(b) => Some(*b),
            _ => None,
        })
        .unwrap_or(true);
    
    Ok(Json(EnableValue { enabled }))
}

pub async fn set_riaa_enable(
    State(state): State<Arc<NodeState>>,
    Json(enable_value): Json<EnableValue>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state.set_parameter("riaa:RIAA Enable", ParameterValue::Bool(enable_value.enabled))?;
    
    Ok(Json(serde_json::json!({
        "status": "ok",
        "enabled": enable_value.enabled
    })))
}

pub async fn get_declick_enable(State(state): State<Arc<NodeState>>) -> Result<Json<EnableValue>, ApiError> {
    let params = state.get_params()?;
    
    let enabled = params.get("riaa:Declick Enable")
        .and_then(|v| match v {
            ParameterValue::Bool(b) => Some(*b),
            _ => None,
        })
        .unwrap_or(false);
    
    Ok(Json(EnableValue { enabled }))
}

pub async fn set_declick_enable(
    State(state): State<Arc<NodeState>>,
    Json(enable_value): Json<EnableValue>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state.set_parameter("riaa:Declick Enable", ParameterValue::Bool(enable_value.enabled))?;
    
    Ok(Json(serde_json::json!({
        "status": "ok",
        "enabled": enable_value.enabled
    })))
}

pub async fn get_spike_config(State(state): State<Arc<NodeState>>) -> Result<Json<SpikeConfig>, ApiError> {
    let params = state.get_params()?;
    
    let threshold_db = params.get("riaa:Spike Threshold (dB)")
        .and_then(|v| match v {
            ParameterValue::Float(f) => Some(*f),
            _ => None,
        })
        .unwrap_or(20.0);
    
    let width_ms = params.get("riaa:Spike Width (ms)")
        .and_then(|v| match v {
            ParameterValue::Float(f) => Some(*f),
            _ => None,
        })
        .unwrap_or(1.0);
    
    Ok(Json(SpikeConfig {
        threshold_db,
        width_ms,
    }))
}

pub async fn set_spike_config(
    State(state): State<Arc<NodeState>>,
    Json(spike_config): Json<SpikeConfig>,
) -> Result<Json<serde_json::Value>, ApiError> {
    use std::collections::HashMap;
    let mut params = HashMap::new();
    
    params.insert("riaa:Spike Threshold (dB)".to_string(), ParameterValue::Float(spike_config.threshold_db));
    params.insert("riaa:Spike Width (ms)".to_string(), ParameterValue::Float(spike_config.width_ms));
    
    state.set_parameters(params)?;
    
    Ok(Json(serde_json::json!({
        "status": "ok",
        "threshold_db": spike_config.threshold_db,
        "width_ms": spike_config.width_ms
    })))
}

pub async fn get_notch_config(State(state): State<Arc<NodeState>>) -> Result<Json<NotchConfig>, ApiError> {
    let params = state.get_params()?;
    
    let enabled = params.get("riaa:Notch Filter Enable")
        .and_then(|v| match v {
            ParameterValue::Bool(b) => Some(*b),
            _ => None,
        })
        .unwrap_or(false);
    
    let frequency_hz = params.get("riaa:Notch Frequency (Hz)")
        .and_then(|v| match v {
            ParameterValue::Float(f) => Some(*f),
            _ => None,
        })
        .unwrap_or(250.0);
    
    let q_factor = params.get("riaa:Notch Q Factor")
        .and_then(|v| match v {
            ParameterValue::Float(f) => Some(*f),
            _ => None,
        })
        .unwrap_or(25.0);
    
    Ok(Json(NotchConfig {
        enabled,
        frequency_hz,
        q_factor,
    }))
}

pub async fn set_notch_config(
    State(state): State<Arc<NodeState>>,
    Json(notch_config): Json<NotchConfig>,
) -> Result<Json<serde_json::Value>, ApiError> {
    use std::collections::HashMap;
    let mut params = HashMap::new();
    
    params.insert("riaa:Notch Filter Enable".to_string(), ParameterValue::Bool(notch_config.enabled));
    params.insert("riaa:Notch Frequency (Hz)".to_string(), ParameterValue::Float(notch_config.frequency_hz));
    params.insert("riaa:Notch Q Factor".to_string(), ParameterValue::Float(notch_config.q_factor));
    
    state.set_parameters(params)?;
    
    Ok(Json(serde_json::json!({
        "status": "ok",
        "enabled": notch_config.enabled,
        "frequency_hz": notch_config.frequency_hz,
        "q_factor": notch_config.q_factor
    })))
}

pub async fn set_default(State(state): State<Arc<NodeState>>) -> Result<Json<serde_json::Value>, ApiError> {
    use std::collections::HashMap;
    let mut params = HashMap::new();
    
    // Set defaults: 0dB gain, no subsonic filter, no declick, no RIAA enabled
    params.insert("riaa:Gain (dB)".to_string(), ParameterValue::Float(0.0));
    params.insert("riaa:Subsonic Filter".to_string(), ParameterValue::Int(0));
    params.insert("riaa:RIAA Enable".to_string(), ParameterValue::Bool(false));
    params.insert("riaa:Declick Enable".to_string(), ParameterValue::Bool(false));
    
    state.set_parameters(params)?;
    
    Ok(Json(serde_json::json!({
        "status": "ok",
        "message": "RIAA parameters reset to defaults"
    })))
}

// Create router for RIAA endpoints
pub fn create_router(state: Arc<NodeState>) -> Router {
    Router::new()
        .route("/api/v1/module/riaa/config", get(get_config))
        .route("/api/v1/module/riaa/gain", get(get_gain).put(set_gain))
        .route("/api/v1/module/riaa/subsonic", get(get_subsonic_filter).put(set_subsonic_filter))
        .route("/api/v1/module/riaa/riaa-enable", get(get_riaa_enable).put(set_riaa_enable))
        .route("/api/v1/module/riaa/declick", get(get_declick_enable).put(set_declick_enable))
        .route("/api/v1/module/riaa/spike", get(get_spike_config).put(set_spike_config))
        .route("/api/v1/module/riaa/notch", get(get_notch_config).put(set_notch_config))
        .route("/api/v1/module/riaa/set-default", put(set_default))
        .with_state(state)
}
