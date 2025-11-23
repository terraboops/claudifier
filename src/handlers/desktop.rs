//! Desktop notification handler.
//!
//! Sends system notifications using the platform's native notification system.

use crate::error::NotificationError;
use crate::event::Event;
use crate::handlers::{Handler, HandlerResult};
use async_trait::async_trait;
use notify_rust::{Notification, Timeout, Urgency};
use serde_json::Value;
use std::collections::HashMap;

/// Handler for desktop notifications.
pub struct DesktopHandler;

#[async_trait]
impl Handler for DesktopHandler {
    fn handler_type(&self) -> &str {
        "desktop"
    }

    async fn handle(&self, event: &Event, config: &HashMap<String, Value>) -> HandlerResult<()> {
        // Extract configuration with defaults
        let summary = get_string(config, "summary", "Claude Code Notification");
        let body = render_template(config.get("body"), event);
        let timeout_ms = get_u32(config, "timeout", 5000);
        let urgency = get_string(config, "urgency", "normal");

        // Build and send notification
        let mut notification = Notification::new();
        notification
            .appname("Claude Code")
            .summary(&summary)
            .body(&body)
            .timeout(Timeout::Milliseconds(timeout_ms));

        // Set urgency
        notification.urgency(match urgency.as_str() {
            "low" => Urgency::Low,
            "critical" => Urgency::Critical,
            _ => Urgency::Normal,
        });

        notification
            .show()
            .map_err(|e| NotificationError::SendFailed(format!("Failed to send desktop notification: {}", e)))?;

        Ok(())
    }
}

/// Helper to get string from config with default.
fn get_string(config: &HashMap<String, Value>, key: &str, default: &str) -> String {
    config
        .get(key)
        .and_then(|v| v.as_str())
        .unwrap_or(default)
        .to_string()
}

/// Helper to get u32 from config with default.
fn get_u32(config: &HashMap<String, Value>, key: &str, default: u32) -> u32 {
    config
        .get(key)
        .and_then(|v| v.as_u64())
        .map(|v| v as u32)
        .unwrap_or(default)
}

/// Renders a template string with event data.
fn render_template(template: Option<&Value>, event: &Event) -> String {
    let template_str = match template {
        Some(Value::String(s)) => s,
        _ => return format!("{:?}", event.data),
    };

    let mut result = template_str.clone();

    // Simple template substitution: {{field_name}}
    for (key, value) in &event.data {
        let placeholder = format!("{{{{{}}}}}", key);
        if result.contains(&placeholder) {
            let value_str = match value {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            result = result.replace(&placeholder, &value_str);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::Event;

    #[test]
    fn test_template_rendering() {
        let event = Event::from_json(r#"{"status": "success", "tool": "bash"}"#).unwrap();
        let template = Value::String("Tool {{tool}} completed with status {{status}}".to_string());

        let result = render_template(Some(&template), &event);
        assert_eq!(result, "Tool bash completed with status success");
    }

    #[test]
    fn test_get_string() {
        let mut config = HashMap::new();
        config.insert("key".to_string(), Value::String("value".to_string()));

        assert_eq!(get_string(&config, "key", "default"), "value");
        assert_eq!(get_string(&config, "missing", "default"), "default");
    }
}
