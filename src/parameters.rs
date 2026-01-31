use anyhow::{anyhow, Result};
use libspa::param::ParamType;
use libspa::pod::{
    deserialize::PodDeserializer, serialize::PodSerializer, Object, Pod, Property, PropertyFlags, Value,
};
use libspa_sys;
use pipewire as pw;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::io::Cursor;
use std::rc::Rc;

/// Enum for parameter values (simplified from libspa::pod::Value)
#[derive(Debug, Clone, PartialEq)]
pub enum ParameterValue {
    Bool(bool),
    Int(i32),
    Float(f32),
    String(String),
}

impl ParameterValue {
    /// Convert to libspa::pod::Value
    pub fn to_pod_value(&self) -> Value {
        match self {
            ParameterValue::Bool(b) => Value::Bool(*b),
            ParameterValue::Int(i) => Value::Int(*i),
            ParameterValue::Float(f) => Value::Float(*f),
            ParameterValue::String(s) => Value::String(s.clone()),
        }
    }

    /// Convert from libspa::pod::Value
    pub fn from_pod_value(value: &Value) -> Option<Self> {
        match value {
            Value::Bool(b) => Some(ParameterValue::Bool(*b)),
            Value::Int(i) => Some(ParameterValue::Int(*i)),
            Value::Float(f) => Some(ParameterValue::Float(*f)),
            Value::String(s) => Some(ParameterValue::String(s.clone())),
            _ => None,
        }
    }

    /// Parse from string
    pub fn parse_from_string(s: &str) -> Result<Self> {
        if s == "true" {
            Ok(ParameterValue::Bool(true))
        } else if s == "false" {
            Ok(ParameterValue::Bool(false))
        } else if let Ok(f) = s.parse::<f32>() {
            Ok(ParameterValue::Float(f))
        } else if let Ok(i) = s.parse::<i32>() {
            Ok(ParameterValue::Int(i))
        } else {
            Ok(ParameterValue::String(s.to_string()))
        }
    }

    /// Convert to display string
    pub fn to_string(&self) -> String {
        match self {
            ParameterValue::Bool(b) => b.to_string(),
            ParameterValue::Int(i) => i.to_string(),
            ParameterValue::Float(f) => f.to_string(),
            ParameterValue::String(s) => s.clone(),
        }
    }
}

/// Get all parameters from a node
pub fn get_all_params(
    node: &pw::node::Node,
    mainloop: &pw::main_loop::MainLoopRc,
) -> Result<HashMap<String, ParameterValue>> {
    let params_data = Rc::new(RefCell::new(HashMap::new()));
    let params_for_closure = params_data.clone();
    let done = Rc::new(Cell::new(false));
    let done_clone = done.clone();
    let mainloop_clone = mainloop.clone();

    let _listener = node
        .add_listener_local()
        .param(move |_, param_type, _, _, param| {
            if param_type != ParamType::Props {
                return;
            }

            if let Some(pod) = param {
                if let Ok((_, value)) = PodDeserializer::deserialize_from::<Value>(pod.as_bytes()) {
                    if let Value::Object(obj) = value {
                        // Look for the params property (key 524289)
                        for prop in obj.properties {
                            if prop.key == libspa_sys::SPA_PROP_params {
                                // This contains a Struct with alternating String/Value pairs
                                if let Value::Struct(fields) = prop.value {
                                    let mut i = 0;
                                    while i + 1 < fields.len() {
                                        if let (Value::String(name), value) = (&fields[i], &fields[i + 1]) {
                                            if let Some(param_value) = ParameterValue::from_pod_value(value) {
                                                params_for_closure.borrow_mut().insert(name.clone(), param_value);
                                            }
                                        }
                                        i += 2;
                                    }
                                }
                                break;
                            }
                        }
                    }
                }
            }
            done_clone.set(true);
            mainloop_clone.quit();
        })
        .register();

    // Request Props params
    node.enum_params(0, Some(ParamType::Props), 0, u32::MAX);

    // Run mainloop until we get the response
    mainloop.run();

    if !done.get() {
        return Err(anyhow!("Timeout waiting for parameters"));
    }

    // Extract the data before returning
    let result = params_data.borrow().clone();
    Ok(result)
}

/// Set a parameter on a node
pub fn set_param(
    node: &pw::node::Node,
    mainloop: &pw::main_loop::MainLoopRc,
    param_name: &str,
    value: ParameterValue,
) -> Result<()> {
    // Get all current parameters to verify the parameter exists
    let params = get_all_params(node, mainloop)?;

    // Ensure full parameter name
    let full_name = if param_name.starts_with("speakereq") {
        param_name.to_string()
    } else {
        format!("speakereq2x2:{}", param_name)
    };

    if !params.contains_key(&full_name) {
        return Err(anyhow!("Parameter '{}' not found", param_name));
    }

    // Build Struct with just the single parameter (name, value)
    let struct_fields = vec![
        Value::String(full_name.clone()),
        value.to_pod_value(),
    ];

    // Create Props object with params property containing the Struct
    let properties = vec![Property {
        key: libspa_sys::SPA_PROP_params,
        flags: PropertyFlags::empty(),
        value: Value::Struct(struct_fields),
    }];

    let pod_object = Object {
        type_: libspa_sys::SPA_TYPE_OBJECT_Props,
        id: libspa_sys::SPA_PARAM_Props,
        properties,
    };

    // Serialize POD
    let (values, _) = PodSerializer::serialize(
        Cursor::new(Vec::new()),
        &Value::Object(pod_object),
    )?;

    let bytes = values.into_inner();
    let pod = Pod::from_bytes(&bytes).ok_or_else(|| anyhow!("Failed to create POD"))?;

    // Set parameter on node
    node.set_param(ParamType::Props, 0, pod);

    // The mainloop needs to run briefly to process the command
    // Use a short iteration to flush pending messages
    let done = Rc::new(Cell::new(false));
    let done_clone = done.clone();
    let ml_clone = mainloop.clone();
    let _timer = mainloop.loop_().add_timer(move |_| {
        done_clone.set(true);
        ml_clone.quit();
    });
    _timer.update_timer(
        Some(std::time::Duration::from_millis(50)),
        None
    );

    mainloop.run();

    Ok(())
}

/// Set a parameter from a string value
pub fn set_param_from_string(
    node: &pw::node::Node,
    mainloop: &pw::main_loop::MainLoopRc,
    param_name: &str,
    value_str: &str,
) -> Result<()> {
    let value = ParameterValue::parse_from_string(value_str)?;
    set_param(node, mainloop, param_name, value)
}
