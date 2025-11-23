//! Notification hook implementation.

use super::{HandlerOutcome, Hook};
use serde_json::{json, Value};

/// Handler for Notification hooks.
///
/// These hooks fire when Claude Code wants to notify about an event.
/// We return an empty object {} to indicate passive observation.
pub struct NotificationHook;

impl Hook for NotificationHook {
    fn hook_type(&self) -> &str {
        "Notification"
    }

    fn generate_response(&self, _outcomes: &[HandlerOutcome]) -> Value {
        // Return empty object - just observing notifications
        // Could add systemMessage here if needed to show warnings to user
        json!({})
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_hook_response() {
        let hook = NotificationHook;
        assert_eq!(hook.hook_type(), "Notification");

        let response = hook.generate_response(&[]);
        assert_eq!(response, json!({}));
    }
}
