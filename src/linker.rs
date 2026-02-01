use regex::Regex;
use serde::{Deserialize, Serialize};

/// Log level for rule execution messages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl Default for LogLevel {
    fn default() -> Self {
        LogLevel::Info
    }
}

/// Default log level for errors
fn default_error_level() -> LogLevel {
    LogLevel::Error
}

/// Link operation type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LinkType {
    Link,
    Unlink,
}

/// Node identifier - can use node.name, node.nick, or object.path with wildcard support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeIdentifier {
    #[serde(rename = "node.name")]
    pub node_name: Option<String>,
    #[serde(rename = "node.nick")]
    pub node_nick: Option<String>,
    #[serde(rename = "object.path")]
    pub object_path: Option<String>,
}

impl NodeIdentifier {
    /// Check if a node matches this identifier using HashMap properties
    pub fn matches_properties(&self, props: &std::collections::HashMap<String, String>) -> bool {
        if let Some(ref pattern) = self.node_name {
            if let Some(name) = props.get("node.name") {
                if regex_match(pattern, name) {
                    return true;
                }
            }
        }
        
        if let Some(ref pattern) = self.node_nick {
            if let Some(nick) = props.get("node.nick") {
                if regex_match(pattern, nick) {
                    return true;
                }
            }
        }
        
        if let Some(ref pattern) = self.object_path {
            if let Some(path) = props.get("object.path") {
                if regex_match(pattern, path) {
                    return true;
                }
            }
        }
        
        false
    }
}

/// A link rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkRule {
    /// Name of the link rule (used for the created link objects)
    pub name: String,
    pub source: NodeIdentifier,
    pub destination: NodeIdentifier,
    #[serde(rename = "type")]
    pub link_type: LinkType,
    /// Whether to apply this rule at startup (default: true)
    #[serde(default = "default_link_at_startup")]
    pub link_at_startup: bool,
    /// How often to check and relink in seconds. 0 = link once only (default: 0)
    #[serde(default)]
    pub relink_every: u64,
    /// Log level for normal operations (link created, already exists, etc.) - default: info
    #[serde(default)]
    pub info_level: LogLevel,
    /// Log level for errors (node not found, can't create link, etc.) - default: error
    #[serde(default = "default_error_level")]
    pub error_level: LogLevel,
}

fn default_link_at_startup() -> bool {
    true
}

/// Match a string against a regex pattern
fn regex_match(pattern: &str, text: &str) -> bool {
    if let Ok(re) = Regex::new(pattern) {
        re.is_match(text)
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_regex_match() {
        // Test basic regex patterns
        assert!(regex_match("^test.*", "test123"));
        assert!(regex_match(".*test$", "mytest"));
        assert!(regex_match(".*test.*", "myteststring"));
        assert!(regex_match("^test$", "test"));
        assert!(!regex_match("^test$", "test123"));
        assert!(regex_match("^node\\.", "node.input"));
        
        // Test single character patterns
        assert!(regex_match("^test.$", "test1"));
        assert!(regex_match("^test.$", "testa"));
        assert!(!regex_match("^test.$", "test12"));
        assert!(regex_match("^speakereq.x.\\.output$", "speakereq2x2.output"));
        assert!(regex_match("^speakereq.x.\\.output$", "speakereq4x4.output"));
        
        // Test complex patterns
        assert!(regex_match("alsa.*sndrpihifiberry.*playback", "alsa:acp:sndrpihifiberry:1:playback"));
        assert!(regex_match("alsa:.*:sndrpihifiberry:.*:playback", "alsa:acp:sndrpihifiberry:1:playback"));
        assert!(regex_match("^test..*", "test1234"));
    }
}
