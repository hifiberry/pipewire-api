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
