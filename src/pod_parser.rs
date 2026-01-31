use std::collections::HashMap;
use serde_json::Value as JsonValue;
use libspa::pod::Pod;
use libspa::pod::deserialize::PodDeserializer;
use libspa::pod::Value as PodValue;

/// Parse a SPA Pod into a JSON-friendly HashMap
/// This attempts to extract common properties like volume, mute, channelVolumes, etc.
pub fn parse_props_pod(pod: &Pod) -> HashMap<String, JsonValue> {
    let mut result = HashMap::new();
    
    // Deserialize the Pod
    if let Ok((_, value)) = PodDeserializer::deserialize_from::<PodValue>(pod.as_bytes()) {
        if let PodValue::Object(obj) = value {
            // Iterate through all properties
            for prop in obj.properties {
                let key = prop.key;
                
                if let Some(json_value) = pod_value_to_json(&prop.value) {
                    // Store with numeric key
                    result.insert(format!("prop_{}", key), json_value.clone());
                    
                    // Also map known property IDs to friendly names
                    // From /usr/include/spa-0.2/spa/param/props.h
                    match key {
                        65539 => { // SPA_PROP_volume
                            result.insert("volume".to_string(), json_value);
                        }
                        65540 => { // SPA_PROP_mute
                            result.insert("mute".to_string(), json_value);
                        }
                        65544 => { // SPA_PROP_channelVolumes
                            result.insert("channelVolumes".to_string(), json_value);
                        }
                        65545 => { // SPA_PROP_volumeBase
                            result.insert("volumeBase".to_string(), json_value);
                        }
                        65546 => { // SPA_PROP_volumeStep
                            result.insert("volumeStep".to_string(), json_value);
                        }
                        65547 => { // SPA_PROP_channelMap
                            result.insert("channelMap".to_string(), json_value);
                        }
                        65548 => { // SPA_PROP_monitorMute
                            result.insert("monitorMute".to_string(), json_value);
                        }
                        65549 => { // SPA_PROP_monitorVolumes
                            result.insert("monitorVolumes".to_string(), json_value);
                        }
                        65551 => { // SPA_PROP_softMute
                            result.insert("softMute".to_string(), json_value);
                        }
                        65552 => { // SPA_PROP_softVolumes
                            result.insert("softVolumes".to_string(), json_value);
                        }
                        524289 => { // SPA_PROP_params (container for plugin params)
                            result.insert("params_struct".to_string(), json_value);
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    
    result
}

/// Convert a PodValue to a JsonValue
fn pod_value_to_json(value: &PodValue) -> Option<JsonValue> {
    match value {
        PodValue::Bool(b) => Some(JsonValue::Bool(*b)),
        PodValue::Int(i) => Some(JsonValue::Number((*i).into())),
        PodValue::Long(l) => Some(JsonValue::Number((*l).into())),
        PodValue::Float(f) => {
            serde_json::Number::from_f64(*f as f64).map(JsonValue::Number)
        }
        PodValue::Double(d) => {
            serde_json::Number::from_f64(*d).map(JsonValue::Number)
        }
        PodValue::String(s) => {
            // Trim null terminators from C strings
            let trimmed = s.trim_end_matches('\0');
            Some(JsonValue::String(trimmed.to_string()))
        }
        PodValue::Id(id) => Some(JsonValue::Number((id.0).into())),
        PodValue::ValueArray(arr) => {
            // ValueArray is an enum with different typed Vec variants
            match arr {
                libspa::pod::ValueArray::None(_) => Some(JsonValue::Array(vec![])),
                libspa::pod::ValueArray::Bool(vec) => {
                    Some(JsonValue::Array(vec.iter().map(|&b| JsonValue::Bool(b)).collect()))
                }
                libspa::pod::ValueArray::Id(vec) => {
                    Some(JsonValue::Array(vec.iter().map(|id| JsonValue::Number((id.0).into())).collect()))
                }
                libspa::pod::ValueArray::Int(vec) => {
                    Some(JsonValue::Array(vec.iter().map(|&i| JsonValue::Number(i.into())).collect()))
                }
                libspa::pod::ValueArray::Long(vec) => {
                    Some(JsonValue::Array(vec.iter().map(|&l| JsonValue::Number(l.into())).collect()))
                }
                libspa::pod::ValueArray::Float(vec) => {
                    let values: Vec<JsonValue> = vec.iter()
                        .filter_map(|&f| serde_json::Number::from_f64(f as f64).map(JsonValue::Number))
                        .collect();
                    Some(JsonValue::Array(values))
                }
                libspa::pod::ValueArray::Double(vec) => {
                    let values: Vec<JsonValue> = vec.iter()
                        .filter_map(|&d| serde_json::Number::from_f64(d).map(JsonValue::Number))
                        .collect();
                    Some(JsonValue::Array(values))
                }
                libspa::pod::ValueArray::Rectangle(vec) => {
                    // Convert rectangles to objects with width/height
                    let values: Vec<JsonValue> = vec.iter()
                        .map(|r| {
                            serde_json::json!({
                                "width": r.width,
                                "height": r.height
                            })
                        })
                        .collect();
                    Some(JsonValue::Array(values))
                }
                libspa::pod::ValueArray::Fraction(vec) => {
                    // Convert fractions to objects with num/denom
                    let values: Vec<JsonValue> = vec.iter()
                        .map(|f| {
                            serde_json::json!({
                                "num": f.num,
                                "denom": f.denom
                            })
                        })
                        .collect();
                    Some(JsonValue::Array(values))
                }
                libspa::pod::ValueArray::Fd(vec) => {
                    // File descriptors - just show the count
                    Some(JsonValue::String(format!("Array of {} file descriptors", vec.len())))
                }
            }
        }
        PodValue::Struct(fields) => {
            let values: Vec<JsonValue> = fields.iter()
                .filter_map(|v| pod_value_to_json(v))
                .collect();
            if !values.is_empty() {
                Some(JsonValue::Array(values))
            } else {
                None
            }
        }
        PodValue::Object(obj) => {
            // Handle nested objects by parsing them recursively
            let mut nested = serde_json::Map::new();
            for prop in &obj.properties {
                if let Some(json_value) = pod_value_to_json(&prop.value) {
                    nested.insert(format!("prop_{}", prop.key), json_value.clone());
                    
                    // Also add friendly names for known properties
                    match prop.key {
                        65539 => { nested.insert("volume".to_string(), json_value); }
                        65540 => { nested.insert("mute".to_string(), json_value); }
                        65544 => { nested.insert("channelVolumes".to_string(), json_value); }
                        65545 => { nested.insert("volumeBase".to_string(), json_value); }
                        65546 => { nested.insert("volumeStep".to_string(), json_value); }
                        65547 => { nested.insert("channelMap".to_string(), json_value); }
                        _ => {}
                    }
                }
            }
            Some(JsonValue::Object(nested))
        }
        _ => None,
    }
}
