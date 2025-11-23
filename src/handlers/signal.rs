//! Signal messenger notification handler.
//!
//! Sends messages via Signal using signal-cli.

use crate::error::NotificationError;
use crate::event::Event;
use crate::handlers::{Handler, HandlerResult};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use tokio::process::Command;

/// Handler for Signal notifications.
pub struct SignalHandler;

#[async_trait]
impl Handler for SignalHandler {
    fn handler_type(&self) -> &str {
        "signal"
    }

    async fn handle(&self, event: &Event, config: &HashMap<String, Value>) -> HandlerResult<()> {
        // Get recipient from config
        let recipient = config
            .get("recipient")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                NotificationError::InvalidConfig("Signal handler requires 'recipient' configuration".to_string())
            })?;

        // Get message template or use default
        let message = render_message(config.get("message"), event);

        // Get optional signal-cli path (defaults to 'signal-cli' in PATH)
        let signal_cli_path = config
            .get("signal_cli_path")
            .and_then(|v| v.as_str())
            .unwrap_or("signal-cli");

        // Get optional account (sender number)
        let account = config.get("account").and_then(|v| v.as_str());

        // Send the message
        send_signal_message(signal_cli_path, account, recipient, &message).await?;

        Ok(())
    }
}

async fn send_signal_message(
    signal_cli_path: &str,
    account: Option<&str>,
    recipient: &str,
    message: &str,
) -> HandlerResult<()> {
    let mut cmd = Command::new(signal_cli_path);

    // Add account if specified
    if let Some(acc) = account {
        cmd.arg("-a").arg(acc);
    }

    // Build the command
    cmd.arg("send").arg("-m").arg(message).arg(recipient);

    // Execute the command
    let output = cmd
        .output()
        .await
        .map_err(|e| NotificationError::Handler(format!("Failed to execute signal-cli: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(NotificationError::Handler(format!("signal-cli failed: {}", stderr)));
    }

    Ok(())
}

fn render_message(template: Option<&Value>, event: &Event) -> String {
    let template_str = match template {
        Some(Value::String(s)) => s,
        _ => return format!("Claude Code Event: {:?}", event.data),
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

    #[test]
    fn test_handler_type() {
        let handler = SignalHandler;
        assert_eq!(handler.handler_type(), "signal");
    }

    #[test]
    fn test_render_message() {
        let event = Event::from_json(r#"{"status": "success", "task": "build"}"#).unwrap();
        let template = Value::String("Task {{task}} completed: {{status}}".to_string());

        let result = render_message(Some(&template), &event);
        assert_eq!(result, "Task build completed: success");
    }

    #[tokio::test]
    async fn test_missing_recipient() {
        let handler = SignalHandler;
        let event = Event::from_json(r#"{"test": "data"}"#).unwrap();
        let config = HashMap::new();

        let result = handler.handle(&event, &config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("requires 'recipient'"));
    }
}
