use axum::{
    extract::State,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::collections::HashMap;
use crate::api_server::{ApiError, NodeState};
use crate::parameters::ParameterValue;

/// Shared state containing both module states
#[derive(Clone)]
pub struct SettingsState {
    pub speakereq: Arc<NodeState>,
    pub riaa: Arc<NodeState>,
}

/// Complete settings state for all modules
#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
    pub version: String,
    pub speakereq: Option<crate::speakereq::StatusResponse>,
    pub riaa: Option<crate::riaa::RiaaConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveResponse {
    pub success: bool,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RestoreResponse {
    pub success: bool,
    pub message: String,
    pub modules_restored: Vec<String>,
}

/// Get the settings file path
pub fn get_settings_path() -> Result<PathBuf, ApiError> {
    let home = std::env::var("HOME")
        .map_err(|_| ApiError::Internal("HOME environment variable not set".to_string()))?;
    
    let state_dir = PathBuf::from(home).join(".state").join("pipewire-api");
    
    // Create directory if it doesn't exist
    if !state_dir.exists() {
        fs::create_dir_all(&state_dir)
            .map_err(|e| ApiError::Internal(format!("Failed to create state directory: {}", e)))?;
    }
    
    Ok(state_dir.join("settings.json"))
}

/// Save current settings to disk
pub async fn save_settings(
    State(state): State<SettingsState>,
) -> Result<Json<SaveResponse>, ApiError> {
    let path = get_settings_path()?;
    
    // Get cached parameters from each module state
    let speakereq_status = match state.speakereq.get_params() {
        Ok(_params) => {
            // Use get_status which reads from cached params
            match crate::speakereq::get_status(State(state.speakereq.clone())).await {
                Ok(Json(status)) => Some(status),
                Err(_) => None,
            }
        }
        Err(_) => None,
    };
    
    let riaa_config = match state.riaa.get_params() {
        Ok(_params) => {
            match crate::riaa::get_config(State(state.riaa.clone())).await {
                Ok(Json(config)) => Some(config),
                Err(_) => None,
            }
        }
        Err(_) => None,
    };
    
    let settings = Settings {
        version: env!("CARGO_PKG_VERSION").to_string(),
        speakereq: speakereq_status,
        riaa: riaa_config,
    };
    
    // Serialize to JSON with pretty formatting
    let json = serde_json::to_string_pretty(&settings)
        .map_err(|e| ApiError::Internal(format!("Failed to serialize settings: {}", e)))?;
    
    // Write to file
    fs::write(&path, json)
        .map_err(|e| ApiError::Internal(format!("Failed to write settings file: {}", e)))?;
    
    Ok(Json(SaveResponse {
        success: true,
        path: path.to_string_lossy().to_string(),
        message: "Settings saved successfully".to_string(),
    }))
}

/// Restore settings from disk by applying saved parameters
pub async fn restore_settings(
    State(state): State<SettingsState>,
) -> Result<Json<RestoreResponse>, ApiError> {
    let path = get_settings_path()?;
    
    if !path.exists() {
        return Err(ApiError::NotFound("No saved settings found".to_string()));
    }
    
    // Read settings file
    let json = fs::read_to_string(&path)
        .map_err(|e| ApiError::Internal(format!("Failed to read settings file: {}", e)))?;
    
    // Deserialize
    let settings: Settings = serde_json::from_str(&json)
        .map_err(|e| ApiError::Internal(format!("Failed to deserialize settings: {}", e)))?;
    
    let mut modules_restored = Vec::new();
    
    // Restore speakereq settings if present
    if let Some(speakereq_settings) = settings.speakereq {
        // Get prefix from cached params
        let params = state.speakereq.get_params()?;
        let prefix = crate::speakereq::get_plugin_prefix(&params);
        
        let mut restore_params = HashMap::new();
        
        // Restore enable and master gain
        restore_params.insert(
            format!("{}:Enable", prefix),
            ParameterValue::Bool(speakereq_settings.enabled)
        );
        restore_params.insert(
            format!("{}:master_gain_db", prefix),
            ParameterValue::Float(speakereq_settings.master_gain_db)
        );
        
        // Restore crossbar matrix
        restore_params.insert(
            format!("{}:xbar_0_to_0", prefix),
            ParameterValue::Float(speakereq_settings.crossbar.input_0_to_output_0)
        );
        restore_params.insert(
            format!("{}:xbar_0_to_1", prefix),
            ParameterValue::Float(speakereq_settings.crossbar.input_0_to_output_1)
        );
        restore_params.insert(
            format!("{}:xbar_1_to_0", prefix),
            ParameterValue::Float(speakereq_settings.crossbar.input_1_to_output_0)
        );
        restore_params.insert(
            format!("{}:xbar_1_to_1", prefix),
            ParameterValue::Float(speakereq_settings.crossbar.input_1_to_output_1)
        );
        
        // Restore input blocks
        for input in &speakereq_settings.inputs {
            let gain_key = format!("{}:{}_gain_db", prefix, input.id);
            restore_params.insert(gain_key, ParameterValue::Float(input.gain_db));
            
            // Restore EQ bands
            for band in &input.eq_bands {
                let type_id = crate::speakereq::eq_type_from_string(&band.eq_type)?;
                let type_key = format!("{}:{}_eq_{}_type", prefix, input.id, band.band);
                let freq_key = format!("{}:{}_eq_{}_f", prefix, input.id, band.band);
                let q_key = format!("{}:{}_eq_{}_q", prefix, input.id, band.band);
                let gain_key = format!("{}:{}_eq_{}_gain", prefix, input.id, band.band);
                let enabled_key = format!("{}:{}_eq_{}_enabled", prefix, input.id, band.band);
                
                restore_params.insert(type_key, ParameterValue::Int(type_id));
                restore_params.insert(freq_key, ParameterValue::Float(band.frequency));
                restore_params.insert(q_key, ParameterValue::Float(band.q));
                restore_params.insert(gain_key, ParameterValue::Float(band.gain));
                restore_params.insert(enabled_key, ParameterValue::Bool(band.enabled));
            }
        }
        
        // Restore output blocks
        for output in &speakereq_settings.outputs {
            let gain_key = format!("{}:{}_gain_db", prefix, output.id);
            restore_params.insert(gain_key, ParameterValue::Float(output.gain_db));
            
            if let Some(delay_ms) = output.delay_ms {
                let delay_key = format!("{}:{}_delay_ms", prefix, output.id);
                restore_params.insert(delay_key, ParameterValue::Float(delay_ms));
            }
            
            // Restore EQ bands
            for band in &output.eq_bands {
                let type_id = crate::speakereq::eq_type_from_string(&band.eq_type)?;
                let type_key = format!("{}:{}_eq_{}_type", prefix, output.id, band.band);
                let freq_key = format!("{}:{}_eq_{}_f", prefix, output.id, band.band);
                let q_key = format!("{}:{}_eq_{}_q", prefix, output.id, band.band);
                let gain_key = format!("{}:{}_eq_{}_gain", prefix, output.id, band.band);
                let enabled_key = format!("{}:{}_eq_{}_enabled", prefix, output.id, band.band);
                
                restore_params.insert(type_key, ParameterValue::Int(type_id));
                restore_params.insert(freq_key, ParameterValue::Float(band.frequency));
                restore_params.insert(q_key, ParameterValue::Float(band.q));
                restore_params.insert(gain_key, ParameterValue::Float(band.gain));
                restore_params.insert(enabled_key, ParameterValue::Bool(band.enabled));
            }
        }
        
        // Apply all speakereq parameters in one batch
        if !restore_params.is_empty() {
            state.speakereq.set_parameters(restore_params)?;
            modules_restored.push("speakereq".to_string());
        }
    }
    
    // Restore RIAA settings if present
    if let Some(riaa_config) = settings.riaa {
        let mut riaa_params = HashMap::new();
        
        riaa_params.insert("riaa:Gain (dB)".to_string(), ParameterValue::Float(riaa_config.gain_db));
        riaa_params.insert("riaa:Subsonic Filter".to_string(), ParameterValue::Int(riaa_config.subsonic_filter));
        riaa_params.insert("riaa:RIAA Enable".to_string(), ParameterValue::Bool(riaa_config.riaa_enable));
        riaa_params.insert("riaa:Declick Enable".to_string(), ParameterValue::Bool(riaa_config.declick_enable));
        riaa_params.insert("riaa:Spike Threshold (dB)".to_string(), ParameterValue::Float(riaa_config.spike_threshold_db));
        riaa_params.insert("riaa:Spike Width (ms)".to_string(), ParameterValue::Float(riaa_config.spike_width_ms));
        riaa_params.insert("riaa:Notch Filter Enable".to_string(), ParameterValue::Bool(riaa_config.notch_filter_enable));
        riaa_params.insert("riaa:Notch Frequency (Hz)".to_string(), ParameterValue::Float(riaa_config.notch_frequency_hz));
        riaa_params.insert("riaa:Notch Q Factor".to_string(), ParameterValue::Float(riaa_config.notch_q_factor));
        
        if !riaa_params.is_empty() {
            state.riaa.set_parameters(riaa_params)?;
            modules_restored.push("riaa".to_string());
        }
    }
    
    Ok(Json(RestoreResponse {
        success: true,
        message: format!("Restored {} modules", modules_restored.len()),
        modules_restored,
    }))
}

/// Create the settings router with both module states
pub fn create_router(
    speakereq_state: Arc<NodeState>,
    riaa_state: Arc<NodeState>,
) -> Router {
    let settings_state = SettingsState {
        speakereq: speakereq_state,
        riaa: riaa_state,
    };
    
    Router::new()
        .route("/api/v1/settings/save", post(save_settings))
        .route("/api/v1/settings/restore", post(restore_settings))
        .with_state(settings_state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_test_env() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_var("HOME", temp_dir.path().to_str().unwrap());
        temp_dir
    }

    #[test]
    fn test_get_settings_path_creates_directory() {
        let _temp_dir = setup_test_env();
        
        let path = get_settings_path().unwrap();
        assert!(path.to_string_lossy().contains(".state/pipewire-api/settings.json"));
        
        // Verify directory was created
        let dir = path.parent().unwrap();
        assert!(dir.exists());
        assert!(dir.is_dir());
    }

    #[test]
    fn test_settings_serialization() {
        let settings = Settings {
            version: "2.0.8".to_string(),
            speakereq: None,
            riaa: None,
        };
        
        let json = serde_json::to_string(&settings).unwrap();
        let deserialized: Settings = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.version, "2.0.8");
        assert!(deserialized.speakereq.is_none());
        assert!(deserialized.riaa.is_none());
    }

    #[test]
    fn test_settings_with_speakereq_serialization() {
        use crate::speakereq::{StatusResponse, CrossbarMatrix, BlockStatus, EqBandStatus};
        
        let crossbar = CrossbarMatrix {
            input_0_to_output_0: 1.0,
            input_0_to_output_1: 0.0,
            input_1_to_output_0: 0.0,
            input_1_to_output_1: 1.0,
        };
        
        let eq_band = EqBandStatus {
            band: 1,
            eq_type: "low_pass".to_string(),
            frequency: 1000.0,
            q: 0.707,
            gain: 0.0,
            enabled: true,
        };
        
        let input = BlockStatus {
            id: "input_0".to_string(),
            block_type: "input".to_string(),
            gain_db: 0.0,
            delay_ms: None,
            eq_bands: vec![eq_band],
        };
        
        let speakereq_status = StatusResponse {
            enabled: true,
            master_gain_db: 0.0,
            crossbar,
            inputs: vec![input],
            outputs: vec![],
        };
        
        let settings = Settings {
            version: "2.0.8".to_string(),
            speakereq: Some(speakereq_status),
            riaa: None,
        };
        
        let json = serde_json::to_string_pretty(&settings).unwrap();
        let deserialized: Settings = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.version, "2.0.8");
        assert!(deserialized.speakereq.is_some());
        
        let speakereq = deserialized.speakereq.unwrap();
        assert_eq!(speakereq.enabled, true);
        assert_eq!(speakereq.master_gain_db, 0.0);
        assert_eq!(speakereq.inputs.len(), 1);
        assert_eq!(speakereq.inputs[0].id, "input_0");
        assert_eq!(speakereq.inputs[0].eq_bands.len(), 1);
        assert_eq!(speakereq.inputs[0].eq_bands[0].eq_type, "low_pass");
    }

    #[test]
    fn test_settings_file_write_and_read() {
        let _temp_dir = setup_test_env();
        
        let settings = Settings {
            version: "2.0.8".to_string(),
            speakereq: None,
            riaa: None,
        };
        
        let path = get_settings_path().unwrap();
        let json = serde_json::to_string_pretty(&settings).unwrap();
        fs::write(&path, json).unwrap();
        
        assert!(path.exists());
        
        let read_json = fs::read_to_string(&path).unwrap();
        let deserialized: Settings = serde_json::from_str(&read_json).unwrap();
        
        assert_eq!(deserialized.version, "2.0.8");
    }

    #[test]
    fn test_settings_path_format() {
        let _temp_dir = setup_test_env();
        
        let path = get_settings_path().unwrap();
        let path_str = path.to_string_lossy();
        
        assert!(path_str.ends_with(".state/pipewire-api/settings.json"));
    }

    #[test]
    fn test_empty_settings_json_structure() {
        let settings = Settings {
            version: "2.0.8".to_string(),
            speakereq: None,
            riaa: None,
        };
        
        let json = serde_json::to_string_pretty(&settings).unwrap();
        
        assert!(json.contains("\"version\""));
        assert!(json.contains("\"speakereq\""));
        assert!(json.contains("\"riaa\""));
        assert!(json.contains("2.0.8"));
    }

    #[test]
    fn test_crossbar_values_preserved() {
        use crate::speakereq::{StatusResponse, CrossbarMatrix};
        
        let crossbar = CrossbarMatrix {
            input_0_to_output_0: 0.5,
            input_0_to_output_1: 0.3,
            input_1_to_output_0: 0.7,
            input_1_to_output_1: 0.9,
        };
        
        let speakereq_status = StatusResponse {
            enabled: true,
            master_gain_db: -3.0,
            crossbar,
            inputs: vec![],
            outputs: vec![],
        };
        
        let settings = Settings {
            version: "2.0.8".to_string(),
            speakereq: Some(speakereq_status),
            riaa: None,
        };
        
        let json = serde_json::to_string(&settings).unwrap();
        let deserialized: Settings = serde_json::from_str(&json).unwrap();
        
        let speakereq = deserialized.speakereq.unwrap();
        assert_eq!(speakereq.crossbar.input_0_to_output_0, 0.5);
        assert_eq!(speakereq.crossbar.input_0_to_output_1, 0.3);
        assert_eq!(speakereq.crossbar.input_1_to_output_0, 0.7);
        assert_eq!(speakereq.crossbar.input_1_to_output_1, 0.9);
    }
    
    #[test]
    fn test_riaa_settings_serialization() {
        use crate::riaa::RiaaConfig;
        
        let riaa_config = RiaaConfig {
            gain_db: 6.0,
            subsonic_filter: 1,
            riaa_enable: true,
            declick_enable: true,
            spike_threshold_db: 15.0,
            spike_width_ms: 2.0,
            notch_filter_enable: true,
            notch_frequency_hz: 60.0,
            notch_q_factor: 20.0,
        };
        
        let settings = Settings {
            version: "2.0.8".to_string(),
            speakereq: None,
            riaa: Some(riaa_config),
        };
        
        let json = serde_json::to_string_pretty(&settings).unwrap();
        let deserialized: Settings = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.version, "2.0.8");
        assert!(deserialized.riaa.is_some());
        
        let riaa = deserialized.riaa.unwrap();
        assert_eq!(riaa.gain_db, 6.0);
        assert_eq!(riaa.subsonic_filter, 1);
        assert_eq!(riaa.riaa_enable, true);
        assert_eq!(riaa.declick_enable, true);
        assert_eq!(riaa.notch_filter_enable, true);
        assert_eq!(riaa.notch_frequency_hz, 60.0);
    }
}
