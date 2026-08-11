use axum::{
    extract::State,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tokio::time::{interval, sleep, Duration};
use tracing::{info, warn};
use crate::api_server::{ApiError, NodeState};
use crate::parameters::ParameterValue;

/// Shared state containing both module states and auto-save state
#[derive(Clone)]
pub struct SettingsState {
    pub speakereq: Arc<NodeState>,
    pub input_processor: Arc<NodeState>,
    pub auto_save: Arc<AutoSaveState>,
}

/// Auto-save state tracking
pub struct AutoSaveState {
    pub last_saved: RwLock<Option<String>>,
    pub interval_secs: u64,
    /// Set once the startup restore has run (or given up). The auto-save task
    /// must not write before that, or it would overwrite the saved settings
    /// with the defaults the modules come up with at boot.
    pub restore_done: AtomicBool,
}

impl AutoSaveState {
    pub fn new(interval_secs: u64) -> Self {
        Self {
            last_saved: RwLock::new(None),
            interval_secs,
            restore_done: AtomicBool::new(false),
        }
    }

    /// Initialize with existing file content if available
    pub fn new_with_file(interval_secs: u64, file_path: &PathBuf) -> Self {
        let initial_content = fs::read_to_string(file_path).ok();
        Self {
            last_saved: RwLock::new(initial_content),
            interval_secs,
            restore_done: AtomicBool::new(false),
        }
    }
}

/// Complete settings state for all modules
#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
    pub version: String,
    pub speakereq: Option<crate::speakereq::StatusResponse>,
    pub input_processor: Option<crate::input_processor::InputProcessorConfig>,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct ResetResponse {
    pub success: bool,
    pub message: String,
    pub modules_reset: Vec<String>,
    pub settings_removed: bool,
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
    
    // Get current settings as JSON
    let json = get_current_settings_json(&state).await?;
    
    // Write to file
    fs::write(&path, &json)
        .map_err(|e| ApiError::Internal(format!("Failed to write settings file: {}", e)))?;
    
    // Log to console for systemd journal
    info!("Settings saved to {}", path.display());
    
    // Update last_saved state
    let mut last_saved = state.auto_save.last_saved.write().await;
    *last_saved = Some(json);
    
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
    let modules_restored = apply_saved_settings(&state).await?;

    Ok(Json(RestoreResponse {
        success: true,
        message: format!("Restored {} modules", modules_restored.len()),
        modules_restored,
    }))
}

/// Reset all modules to their defaults and discard the saved settings.
///
/// Used by the factory reset in the configuration server, so that a reset also
/// clears what this service persists across reboots.
pub async fn reset_settings(
    State(state): State<SettingsState>,
) -> Result<Json<ResetResponse>, ApiError> {
    let mut modules_reset = Vec::new();
    let mut errors = Vec::new();

    // Defaults are applied per module, best effort: a module whose PipeWire
    // node is absent on this device must not fail the whole reset.
    match crate::speakereq::set_default(State(state.speakereq.clone())).await {
        Ok(_) => modules_reset.push("speakereq".to_string()),
        Err(e) => errors.push(format!("speakereq: {:?}", e)),
    }
    match crate::input_processor::set_default(State(state.input_processor.clone())).await {
        Ok(_) => modules_reset.push("input_processor".to_string()),
        Err(e) => errors.push(format!("input_processor: {:?}", e)),
    }

    // Drop the stored settings so a restore cannot bring the old values back
    let path = get_settings_path()?;
    let mut settings_removed = false;
    if path.exists() {
        fs::remove_file(&path)
            .map_err(|e| ApiError::Internal(format!("Failed to remove settings file: {}", e)))?;
        settings_removed = true;
    }
    *state.auto_save.last_saved.write().await = None;

    // Nothing left to restore, so let auto-save resume with the defaults
    state.auto_save.restore_done.store(true, Ordering::SeqCst);

    info!("Settings reset: {} module(s) reset to defaults, settings file removed: {}",
        modules_reset.len(), settings_removed);
    if !errors.is_empty() {
        warn!("Settings reset: {}", errors.join("; "));
    }

    Ok(Json(ResetResponse {
        success: true,
        message: format!("Reset {} module(s) to defaults", modules_reset.len()),
        modules_reset,
        settings_removed,
    }))
}

/// Apply the settings stored on disk to the running modules.
///
/// Returns the names of the modules that were restored. Fails with
/// `ApiError::NotFound` when there is no settings file, and with whatever
/// error the module state reports when the PipeWire node is not (yet) there.
pub async fn apply_saved_settings(state: &SettingsState) -> Result<Vec<String>, ApiError> {
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
    
    // Restore input-processor (RIAA) settings if present
    if let Some(input_processor_config) = settings.input_processor {
        let mut input_processor_params = HashMap::new();

        input_processor_params.insert("input-processor:Gain (dB)".to_string(), ParameterValue::Float(input_processor_config.gain_db));
        input_processor_params.insert("input-processor:Subsonic Filter".to_string(), ParameterValue::Int(input_processor_config.subsonic_filter));
        input_processor_params.insert("input-processor:RIAA Enable".to_string(), ParameterValue::Bool(input_processor_config.riaa_enable));
        input_processor_params.insert("input-processor:Declick Enable".to_string(), ParameterValue::Bool(input_processor_config.declick_enable));
        input_processor_params.insert("input-processor:Spike Threshold (dB)".to_string(), ParameterValue::Float(input_processor_config.spike_threshold_db));
        input_processor_params.insert("input-processor:Spike Width (ms)".to_string(), ParameterValue::Float(input_processor_config.spike_width_ms));
        input_processor_params.insert("input-processor:Notch Filter Enable".to_string(), ParameterValue::Bool(input_processor_config.notch_filter_enable));
        input_processor_params.insert("input-processor:Notch Frequency (Hz)".to_string(), ParameterValue::Float(input_processor_config.notch_frequency_hz));
        input_processor_params.insert("input-processor:Notch Q Factor".to_string(), ParameterValue::Float(input_processor_config.notch_q_factor));

        if !input_processor_params.is_empty() {
            state.input_processor.set_parameters(input_processor_params)?;
            modules_restored.push("input_processor".to_string());
        }
    }
    
    Ok(modules_restored)
}

/// Background task that applies the saved settings once at startup.
///
/// The modules come up with their compiled-in defaults, and the PipeWire node
/// backing them may not exist yet when the API server starts (the filter chain
/// is a separate service), so retry until it shows up. The auto-save task stays
/// parked until this has finished either way.
pub async fn startup_restore_task(state: SettingsState) {
    const RETRY_INTERVAL: Duration = Duration::from_secs(2);
    const MAX_ATTEMPTS: u32 = 60; // ~2 minutes

    let path = match get_settings_path() {
        Ok(path) => path,
        Err(e) => {
            warn!("Startup restore: cannot determine settings path: {:?}", e);
            state.auto_save.restore_done.store(true, Ordering::SeqCst);
            return;
        }
    };

    if !path.exists() {
        info!("Startup restore: no saved settings at {}, nothing to restore", path.display());
        state.auto_save.restore_done.store(true, Ordering::SeqCst);
        return;
    }

    for attempt in 1..=MAX_ATTEMPTS {
        match apply_saved_settings(&state).await {
            Ok(modules) => {
                info!("Startup restore: restored {} module(s) from {}: {}",
                    modules.len(), path.display(), modules.join(", "));
                break;
            }
            Err(e) => {
                if attempt == MAX_ATTEMPTS {
                    warn!("Startup restore: giving up after {} attempts: {:?}", attempt, e);
                } else {
                    tracing::debug!("Startup restore: attempt {} failed ({:?}), retrying", attempt, e);
                    sleep(RETRY_INTERVAL).await;
                }
            }
        }
    }

    state.auto_save.restore_done.store(true, Ordering::SeqCst);
}

/// Get current settings as JSON string (for comparison)
async fn get_current_settings_json(state: &SettingsState) -> Result<String, ApiError> {
    // Get cached parameters from each module state
    let speakereq_status = match state.speakereq.get_params() {
        Ok(_params) => {
            match crate::speakereq::get_status(State(state.speakereq.clone())).await {
                Ok(Json(status)) => Some(status),
                Err(_) => None,
            }
        }
        Err(_) => None,
    };
    
    let input_processor_config = match state.input_processor.get_params() {
        Ok(_params) => {
            match crate::input_processor::get_config(State(state.input_processor.clone())).await {
                Ok(Json(config)) => Some(config),
                Err(_) => None,
            }
        }
        Err(_) => None,
    };

    let settings = Settings {
        version: env!("CARGO_PKG_VERSION").to_string(),
        speakereq: speakereq_status,
        input_processor: input_processor_config,
    };
    
    serde_json::to_string_pretty(&settings)
        .map_err(|e| ApiError::Internal(format!("Failed to serialize settings: {}", e)))
}

/// Would writing `current` over `prev` drop a module we already have data for?
///
/// A module serializes to `null` whenever its PipeWire node is missing - during
/// boot, or while the filter chain restarts. Saving that would throw away the
/// stored settings for good, so such a snapshot is skipped.
fn drops_module_data(prev: &str, current: &str) -> bool {
    // Compared as generic JSON so this keeps working when a module's schema
    // changes, and so a settings file written by an older version still counts.
    let (prev, current) = match (
        serde_json::from_str::<serde_json::Value>(prev),
        serde_json::from_str::<serde_json::Value>(current),
    ) {
        (Ok(prev), Ok(current)) => (prev, current),
        // Unparsable previous content is not worth protecting.
        _ => return false,
    };

    ["speakereq", "input_processor"].iter().any(|module| {
        let had_data = prev.get(module).map_or(false, |v| !v.is_null());
        let lost_data = current.get(module).map_or(true, |v| v.is_null());
        had_data && lost_data
    })
}

/// Background task that auto-saves settings when they change
pub async fn auto_save_task(state: SettingsState) {
    let mut interval = interval(Duration::from_secs(state.auto_save.interval_secs));

    loop {
        interval.tick().await;

        // Don't touch the file before the saved settings have been applied -
        // otherwise the boot-time defaults would overwrite them.
        if !state.auto_save.restore_done.load(Ordering::SeqCst) {
            continue;
        }

        // Get current settings as JSON
        let current_json = match get_current_settings_json(&state).await {
            Ok(json) => json,
            Err(e) => {
                eprintln!("Auto-save: Failed to get current settings: {:?}", e);
                continue;
            }
        };

        // Check if settings have changed
        let mut last_saved = state.auto_save.last_saved.write().await;
        if let Some(prev) = &*last_saved {
            if drops_module_data(prev, &current_json) {
                tracing::debug!("Auto-save: skipping save, a module is currently unavailable");
                continue;
            }
        }
        let has_changed = match &*last_saved {
            Some(prev) => prev != &current_json,
            None => true, // First run, no previous state
        };
        
        if has_changed {
            // Save settings
            match get_settings_path() {
                Ok(path) => {
                    if let Err(e) = fs::write(&path, &current_json) {
                        eprintln!("Auto-save: Failed to write settings: {}", e);
                    } else {
                        *last_saved = Some(current_json);
                        info!("Auto-save: Settings saved to {}", path.display());
                    }
                }
                Err(e) => {
                    eprintln!("Auto-save: Failed to get settings path: {:?}", e);
                }
            }
        }
    }
}

/// Create the settings router with both module states and start auto-save task
pub fn create_router(
    speakereq_state: Arc<NodeState>,
    input_processor_state: Arc<NodeState>,
    auto_save_interval_secs: Option<u64>,
) -> Router {
    // Initialize auto-save state with existing file content if available
    let auto_save = match get_settings_path() {
        Ok(path) if path.exists() => {
            Arc::new(AutoSaveState::new_with_file(auto_save_interval_secs.unwrap_or(10), &path))
        }
        _ => Arc::new(AutoSaveState::new(auto_save_interval_secs.unwrap_or(10))),
    };
    
    let settings_state = SettingsState {
        speakereq: speakereq_state,
        input_processor: input_processor_state,
        auto_save,
    };
    
    // Restore the saved settings before the auto-save task starts writing
    let restore_state = settings_state.clone();
    tokio::spawn(async move {
        startup_restore_task(restore_state).await;
    });

    // Spawn auto-save background task
    let task_state = settings_state.clone();
    tokio::spawn(async move {
        auto_save_task(task_state).await;
    });
    
    Router::new()
        .route("/api/v1/settings/save", post(save_settings))
        .route("/api/v1/settings/restore", post(restore_settings))
        .route("/api/v1/settings/reset", post(reset_settings))
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
    fn test_auto_save_starts_parked_until_restore_ran() {
        let state = AutoSaveState::new(10);
        assert!(!state.restore_done.load(Ordering::SeqCst),
            "auto-save must not write before the startup restore has run");
    }

    #[test]
    fn test_drops_module_data_detects_vanished_module() {
        let with_speakereq = r#"{"version":"1","speakereq":{"whatever":1},"input_processor":null}"#;
        let without = r#"{"version":"1","speakereq":null,"input_processor":null}"#;

        // A snapshot taken while the node is gone must not overwrite real data
        assert!(drops_module_data(with_speakereq, without));
        // The other direction, and unchanged content, are fine to save
        assert!(!drops_module_data(without, with_speakereq));
        assert!(!drops_module_data(without, without));
        assert!(!drops_module_data(with_speakereq, with_speakereq));
    }

    #[test]
    fn test_drops_module_data_ignores_unparsable_previous() {
        let garbage = "not json at all";
        let current = r#"{"version":"1","speakereq":null,"input_processor":null}"#;
        assert!(!drops_module_data(garbage, current));
    }

    #[test]
    fn test_settings_serialization() {
        let settings = Settings {
            version: "2.0.9".to_string(),
            speakereq: None,
            input_processor: None,
        };
        
        let json = serde_json::to_string(&settings).unwrap();
        let deserialized: Settings = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.version, "2.0.9");
        assert!(deserialized.speakereq.is_none());
        assert!(deserialized.input_processor.is_none());
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
            version: "2.0.9".to_string(),
            speakereq: Some(speakereq_status),
            input_processor: None,
        };
        
        let json = serde_json::to_string_pretty(&settings).unwrap();
        let deserialized: Settings = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.version, "2.0.9");
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
            version: "2.0.9".to_string(),
            speakereq: None,
            input_processor: None,
        };
        
        let path = get_settings_path().unwrap();
        let json = serde_json::to_string_pretty(&settings).unwrap();
        fs::write(&path, json).unwrap();
        
        assert!(path.exists());
        
        let read_json = fs::read_to_string(&path).unwrap();
        let deserialized: Settings = serde_json::from_str(&read_json).unwrap();
        
        assert_eq!(deserialized.version, "2.0.9");
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
            version: "2.0.9".to_string(),
            speakereq: None,
            input_processor: None,
        };
        
        let json = serde_json::to_string_pretty(&settings).unwrap();
        
        assert!(json.contains("\"version\""));
        assert!(json.contains("\"speakereq\""));
        assert!(json.contains("\"input_processor\""));
        assert!(json.contains("2.0.9"));
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
            version: "2.0.9".to_string(),
            speakereq: Some(speakereq_status),
            input_processor: None,
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
    fn test_input_processor_settings_serialization() {
        use crate::input_processor::InputProcessorConfig;

        let input_processor_config = InputProcessorConfig {
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
            version: "2.0.9".to_string(),
            speakereq: None,
            input_processor: Some(input_processor_config),
        };

        let json = serde_json::to_string_pretty(&settings).unwrap();
        let deserialized: Settings = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.version, "2.0.9");
        assert!(deserialized.input_processor.is_some());

        let input_processor = deserialized.input_processor.unwrap();
        assert_eq!(input_processor.gain_db, 6.0);
        assert_eq!(input_processor.subsonic_filter, 1);
        assert_eq!(input_processor.riaa_enable, true);
        assert_eq!(input_processor.declick_enable, true);
        assert_eq!(input_processor.notch_filter_enable, true);
        assert_eq!(input_processor.notch_frequency_hz, 60.0);
    }

    #[test]
    fn test_auto_save_state_creation() {
        let auto_save = AutoSaveState::new(10);
        assert_eq!(auto_save.interval_secs, 10);
        
        // Check initial state is None
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let last_saved = auto_save.last_saved.read().await;
            assert!(last_saved.is_none());
        });
    }

    #[test]
    fn test_auto_save_state_update() {
        let auto_save = AutoSaveState::new(5);
        
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // Initially None
            {
                let last_saved = auto_save.last_saved.read().await;
                assert!(last_saved.is_none());
            }
            
            // Update state
            {
                let mut last_saved = auto_save.last_saved.write().await;
                *last_saved = Some("test_json".to_string());
            }
            
            // Verify update
            {
                let last_saved = auto_save.last_saved.read().await;
                assert_eq!(last_saved.as_ref().unwrap(), "test_json");
            }
        });
    }

    #[test]
    fn test_settings_json_comparison() {
        // Test that identical settings produce identical JSON
        let settings1 = Settings {
            version: "2.0.9".to_string(),
            speakereq: None,
            input_processor: None,
        };
        
        let settings2 = Settings {
            version: "2.0.9".to_string(),
            speakereq: None,
            input_processor: None,
        };
        
        let json1 = serde_json::to_string_pretty(&settings1).unwrap();
        let json2 = serde_json::to_string_pretty(&settings2).unwrap();
        
        assert_eq!(json1, json2);
    }

    #[test]
    fn test_settings_json_detects_changes() {
        use crate::input_processor::InputProcessorConfig;

        // Test that different settings produce different JSON
        let settings1 = Settings {
            version: "2.0.9".to_string(),
            speakereq: None,
            input_processor: None,
        };

        let settings2 = Settings {
            version: "2.0.9".to_string(),
            speakereq: None,
            input_processor: Some(InputProcessorConfig {
                gain_db: 5.0,
                subsonic_filter: 1,
                riaa_enable: true,
                declick_enable: false,
                spike_threshold_db: 25.0,
                spike_width_ms: 2.0,
                notch_filter_enable: false,
                notch_frequency_hz: 50.0,
                notch_q_factor: 30.0,
            }),
        };
        
        let json1 = serde_json::to_string_pretty(&settings1).unwrap();
        let json2 = serde_json::to_string_pretty(&settings2).unwrap();
        
        assert_ne!(json1, json2);
    }
}
