//! Common data types for the API

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use serde_json::Value as JsonValue;

// Object type constants
pub const TYPE_NODE: &str = "node";
pub const TYPE_DEVICE: &str = "device";
pub const TYPE_PORT: &str = "port";
pub const TYPE_MODULE: &str = "module";
pub const TYPE_FACTORY: &str = "factory";
pub const TYPE_CLIENT: &str = "client";
pub const TYPE_LINK: &str = "link";

/// Basic PipeWire object info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipeWireObject {
    pub id: u32,
    pub name: String,
    #[serde(rename = "type")]
    pub object_type: String,
}

/// Response for list endpoints
#[derive(Debug, Serialize, Deserialize)]
pub struct ListResponse {
    pub objects: Vec<PipeWireObject>,
}

/// PipeWire object with full properties
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipeWireObjectWithProperties {
    pub id: u32,
    pub name: String,
    #[serde(rename = "type")]
    pub object_type: String,
    pub properties: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic_properties: Option<HashMap<String, JsonValue>>,
}

/// Response for properties endpoints
#[derive(Debug, Serialize, Deserialize)]
pub struct PropertiesResponse {
    pub objects: Vec<PipeWireObjectWithProperties>,
}

/// Device information with optional volume
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: u32,
    pub name: String,
    pub properties: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume: Option<f32>,
}

/// Sink information with optional volume
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SinkInfo {
    pub id: u32,
    pub name: String,
    pub properties: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume: Option<f32>,
}

/// Unified volume info for any volume-controllable object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeInfo {
    pub id: u32,
    pub name: String,
    pub object_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume: Option<f32>,
}

/// Request body for setting volume
#[derive(Debug, Serialize, Deserialize)]
pub struct SetVolumeRequest {
    pub volume: f32,
}

/// Response for volume operations
#[derive(Debug, Serialize, Deserialize)]
pub struct VolumeResponse {
    pub volume: Option<f32>,
}
