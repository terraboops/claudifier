//! Email notification handler.
//!
//! Sends email notifications using SMTP.

use crate::error::NotificationError;
use crate::event::Event;
use crate::handlers::{Handler, HandlerResult};
use async_trait::async_trait;
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use serde_json::Value;
use std::collections::HashMap;

/// Handler for email notifications.
pub struct EmailHandler;

#[async_trait]
impl Handler for EmailHandler {
    fn handler_type(&self) -> &str {
        "email"
    }

    async fn handle(&self, event: &Event, config: &HashMap<String, Value>) -> HandlerResult<()> {
        // Required config
        let to = config
            .get("to")
            .and_then(|v| v.as_str())
            .ok_or_else(|| NotificationError::InvalidConfig("Email handler requires 'to' configuration".to_string()))?;

        let from = config
            .get("from")
            .and_then(|v| v.as_str())
            .ok_or_else(|| NotificationError::InvalidConfig("Email handler requires 'from' configuration".to_string()))?;

        let smtp_server = config
            .get("smtp_server")
            .and_then(|v| v.as_str())
            .ok_or_else(|| NotificationError::InvalidConfig("Email handler requires 'smtp_server' configuration".to_string()))?;

        let smtp_port = config
            .get("smtp_port")
            .and_then(|v| v.as_u64())
            .map(|v| v as u16)
            .unwrap_or(25);

        // Optional config
        let subject = render_template(
            config.get("subject"),
            event,
            "Claude Code Notification",
        );
        let body = render_template(
            config.get("body"),
            event,
            &format!("Event: {:?}", event.data),
        );

        // SMTP credentials (optional)
        let username = config.get("username").and_then(|v| v.as_str());
        let password = config.get("password").and_then(|v| v.as_str());

        // Send email
        send_email(
            from, to, &subject, &body, smtp_server, smtp_port, username, password,
        )
        .await?;

        Ok(())
    }
}

async fn send_email(
    from: &str,
    to: &str,
    subject: &str,
    body: &str,
    smtp_server: &str,
    smtp_port: u16,
    username: Option<&str>,
    password: Option<&str>,
) -> HandlerResult<()> {
    // Build the email
    let email = Message::builder()
        .from(from.parse().map_err(|e| NotificationError::Email(format!("Invalid 'from' address: {}", e)))?)
        .to(to.parse().map_err(|e| NotificationError::Email(format!("Invalid 'to' address: {}", e)))?)
        .subject(subject)
        .header(ContentType::TEXT_PLAIN)
        .body(body.to_string())
        .map_err(|e| NotificationError::Email(format!("Failed to build email: {}", e)))?;

    // Build SMTP transport - use builder_dangerous for local/test servers
    let mut mailer = if smtp_port == 1025 || smtp_server == "localhost" || smtp_server == "127.0.0.1" {
        // Local test server - no TLS
        SmtpTransport::builder_dangerous(smtp_server)
            .port(smtp_port)
    } else {
        // Production server - use relay with TLS
        SmtpTransport::relay(smtp_server)
            .map_err(|e| NotificationError::Email(format!("Failed to connect to SMTP server: {}", e)))?
            .port(smtp_port)
    };

    // Add credentials if provided
    if let (Some(user), Some(pass)) = (username, password) {
        mailer = mailer.credentials(Credentials::new(user.to_string(), pass.to_string()));
    }

    let mailer = mailer.build();

    // Send the email
    mailer
        .send(&email)
        .map_err(|e| NotificationError::Email(format!("Failed to send email: {}", e)))?;

    Ok(())
}

fn render_template(template: Option<&Value>, event: &Event, default: &str) -> String {
    let template_str = match template {
        Some(Value::String(s)) => s,
        _ => return default.to_string(),
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
        let handler = EmailHandler;
        assert_eq!(handler.handler_type(), "email");
    }

    #[test]
    fn test_render_template() {
        let event = Event::from_json(r#"{"status": "success", "task": "build"}"#).unwrap();
        let template = Value::String("Task {{task}}: {{status}}".to_string());

        let result = render_template(Some(&template), &event, "default");
        assert_eq!(result, "Task build: success");
    }

    #[tokio::test]
    async fn test_missing_config() {
        let handler = EmailHandler;
        let event = Event::from_json(r#"{"test": "data"}"#).unwrap();
        let config = HashMap::new();

        let result = handler.handle(&event, &config).await;
        assert!(result.is_err());
    }
}
