//! Webhook notification handler.
//!
//! Sends HTTP POST requests to webhooks (supports Slack, Discord, generic webhooks, etc.).

use crate::error::NotificationError;
use crate::event::Event;
use crate::handlers::{Handler, HandlerResult};
use async_trait::async_trait;
use once_cell::sync::OnceCell;
use reqwest::Client;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;

/// Shared HTTP client with connection pooling.
static HTTP_CLIENT: OnceCell<Client> = OnceCell::new();

/// Initialize or get the HTTP client.
fn get_http_client() -> HandlerResult<&'static Client> {
    HTTP_CLIENT.get_or_try_init(|| {
        Client::builder()
            .timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(10)
            .user_agent(concat!(
                env!("CARGO_PKG_NAME"),
                "/",
                env!("CARGO_PKG_VERSION"),
            ))
            .build()
            .map_err(|e| NotificationError::Network(format!("Failed to build HTTP client: {}", e)))
    })
}

/// Handler for webhook notifications.
pub struct WebhookHandler;

#[async_trait]
impl Handler for WebhookHandler {
    fn handler_type(&self) -> &str {
        "webhook"
    }

    async fn handle(&self, event: &Event, config: &HashMap<String, Value>) -> HandlerResult<()> {
        // Get webhook URL
        let url = config
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                NotificationError::InvalidConfig("Webhook handler requires 'url' configuration".to_string())
            })?;

        // Get payload type (slack, discord, json, or custom)
        let payload_type = config
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("json");

        // Build the payload
        let payload = build_payload(payload_type, event, config)?;

        // Send the webhook
        send_webhook(url, &payload).await?;

        Ok(())
    }
}

fn build_payload(
    payload_type: &str,
    event: &Event,
    config: &HashMap<String, Value>,
) -> HandlerResult<Value> {
    match payload_type {
        "slack" => build_slack_payload(event, config),
        "discord" => build_discord_payload(event, config),
        "json" => build_json_payload(event, config),
        _ => Err(NotificationError::InvalidConfig(format!(
            "Unknown webhook type: {}",
            payload_type
        ))),
    }
}

fn build_slack_payload(event: &Event, config: &HashMap<String, Value>) -> HandlerResult<Value> {
    let text = render_template(config.get("text"), event);

    let mut payload = json!({
        "text": text,
    });

    // Optional: add channel
    if let Some(channel) = config.get("channel").and_then(|v| v.as_str()) {
        payload["channel"] = Value::String(channel.to_string());
    }

    // Optional: add username
    if let Some(username) = config.get("username").and_then(|v| v.as_str()) {
        payload["username"] = Value::String(username.to_string());
    }

    Ok(payload)
}

fn build_discord_payload(event: &Event, config: &HashMap<String, Value>) -> HandlerResult<Value> {
    let content = render_template(config.get("content"), event);

    let mut payload = json!({
        "content": content,
    });

    // Optional: add username
    if let Some(username) = config.get("username").and_then(|v| v.as_str()) {
        payload["username"] = Value::String(username.to_string());
    }

    Ok(payload)
}

fn build_json_payload(event: &Event, config: &HashMap<String, Value>) -> HandlerResult<Value> {
    // Check if custom payload is provided
    if let Some(custom) = config.get("payload") {
        return Ok(render_payload_template(custom, event));
    }

    // Default: send the entire event data
    Ok(event.as_value())
}

/// Recursively render template variables in a JSON value
fn render_payload_template(value: &Value, event: &Event) -> Value {
    match value {
        Value::String(s) => {
            let mut result = s.clone();
            for (key, val) in &event.data {
                let placeholder = format!("{{{{{}}}}}", key);
                if result.contains(&placeholder) {
                    let value_str = match val {
                        Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    result = result.replace(&placeholder, &value_str);
                }
            }
            Value::String(result)
        }
        Value::Object(map) => {
            let mut new_map = serde_json::Map::new();
            for (k, v) in map {
                new_map.insert(k.clone(), render_payload_template(v, event));
            }
            Value::Object(new_map)
        }
        Value::Array(arr) => {
            Value::Array(arr.iter().map(|v| render_payload_template(v, event)).collect())
        }
        other => other.clone(),
    }
}

fn render_template(template: Option<&Value>, event: &Event) -> String {
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

async fn send_webhook(url: &str, payload: &Value) -> HandlerResult<()> {
    let client = get_http_client()?;

    let response = client
        .post(url)
        .json(payload)
        .send()
        .await
        .map_err(|e| NotificationError::Network(format!("Failed to send webhook: {}", e)))?;

    if !response.status().is_success() {
        return Err(NotificationError::Network(format!(
            "Webhook request failed with status: {}",
            response.status()
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handler_type() {
        let handler = WebhookHandler;
        assert_eq!(handler.handler_type(), "webhook");
    }

    #[test]
    fn test_build_slack_payload() {
        let event = Event::from_json(r#"{"status": "success"}"#).unwrap();
        let mut config = HashMap::new();
        config.insert(
            "text".to_string(),
            Value::String("Status: {{status}}".to_string()),
        );

        let payload = build_slack_payload(&event, &config).unwrap();
        assert_eq!(payload["text"], "Status: success");
    }

    #[test]
    fn test_build_discord_payload() {
        let event = Event::from_json(r#"{"task": "build"}"#).unwrap();
        let mut config = HashMap::new();
        config.insert(
            "content".to_string(),
            Value::String("Task: {{task}}".to_string()),
        );

        let payload = build_discord_payload(&event, &config).unwrap();
        assert_eq!(payload["content"], "Task: build");
    }

    #[tokio::test]
    async fn test_missing_url() {
        let handler = WebhookHandler;
        let event = Event::from_json(r#"{"test": "data"}"#).unwrap();
        let config = HashMap::new();

        let result = handler.handle(&event, &config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("requires 'url'"));
    }
}
