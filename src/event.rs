//! Claude Code event types.
//!
//! This module defines the event structure received from Claude Code hooks via stdin.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// A Claude Code event received from stdin.
///
/// Events are flexible JSON objects that can contain any fields.
/// The event type and other metadata are extracted from the JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// The raw JSON value for flexible matching
    #[serde(flatten)]
    pub data: HashMap<String, Value>,
}

impl Event {
    /// Creates a new event from a JSON string.
    ///
    /// # Errors
    ///
    /// Returns an error if the JSON is invalid.
    pub fn from_json(json: &str) -> anyhow::Result<Self> {
        let event = serde_path_to_error::deserialize(&mut serde_json::Deserializer::from_str(json))
            .map_err(|e| anyhow::anyhow!("Failed to parse event JSON: {}", e))?;
        Ok(event)
    }

    /// Gets a field value as a string reference.
    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.data.get(key)?.as_str()
    }

    /// Gets a field value as a string, with nested path support (e.g., "tool.name").
    pub fn get_nested_str(&self, path: &str) -> Option<String> {
        let parts: Vec<&str> = path.split('.').collect();
        let value = Value::Object(
            self.data
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        );

        let mut current = &value;
        for part in parts {
            current = current.get(part)?;
        }

        current.as_str().map(|s| s.to_string())
    }

    /// Gets the entire event data as a reference.
    pub fn as_value(&self) -> Value {
        Value::Object(
            self.data
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_event() {
        let json = r#"{"event_type": "task_complete", "status": "success"}"#;
        let event = Event::from_json(json).unwrap();
        assert_eq!(event.get_str("event_type"), Some("task_complete"));
        assert_eq!(event.get_str("status"), Some("success"));
    }

    #[test]
    fn test_nested_access() {
        let json = r#"{"tool": {"name": "bash", "status": "success"}}"#;
        let event = Event::from_json(json).unwrap();
        assert_eq!(event.get_nested_str("tool.name"), Some("bash".to_string()));
    }

    #[test]
    fn test_invalid_json() {
        let json = r#"{"invalid": }"#;
        assert!(Event::from_json(json).is_err());
    }
}
