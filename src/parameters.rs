use std::fmt;

/// Enum for parameter values used by the API
#[derive(Debug, Clone, PartialEq)]
pub enum ParameterValue {
    Bool(bool),
    Int(i32),
    Float(f32),
    String(String),
}

impl ParameterValue {
    /// Parse from string
    pub fn parse_from_string(s: &str) -> Result<Self, String> {
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

    /// Extract as f32, coercing Int to f32
    pub fn as_float(&self) -> Option<f32> {
        match self {
            ParameterValue::Float(f) => Some(*f),
            ParameterValue::Int(i) => Some(*i as f32),
            _ => None,
        }
    }

    /// Extract as i32 (Int only)
    pub fn as_int(&self) -> Option<i32> {
        match self {
            ParameterValue::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// Extract as bool (Bool directly, Float > 0.5, or Int != 0)
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ParameterValue::Bool(b) => Some(*b),
            ParameterValue::Float(f) => Some(*f > 0.5),
            ParameterValue::Int(i) => Some(*i != 0),
            _ => None,
        }
    }
}

impl fmt::Display for ParameterValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParameterValue::Bool(b) => write!(f, "{}", b),
            ParameterValue::Int(i) => write!(f, "{}", i),
            ParameterValue::Float(v) => write!(f, "{}", v),
            ParameterValue::String(s) => write!(f, "{}", s),
        }
    }
}
