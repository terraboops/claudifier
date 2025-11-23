//! SessionStart and SessionEnd hook implementations.

use super::{HandlerOutcome, Hook};
use serde_json::{json, Value};

/// Handler for SessionStart hooks.
///
/// These hooks fire when a Claude Code session begins.
/// Returns an empty object {} to allow passive observation.
/// Future: Could implement session initialization logic.
pub struct SessionStartHook;

impl Hook for SessionStartHook {
    fn hook_type(&self) -> &str {
        "SessionStart"
    }

    fn generate_response(&self, _outcomes: &[HandlerOutcome]) -> Value {
        // Return empty object - currently just observing
        // Future: Could set up session-specific state or notify external systems
        json!({})
    }
}

/// Handler for SessionEnd hooks.
///
/// These hooks fire when a Claude Code session ends.
/// Returns an empty object {} to allow passive observation.
/// Future: Could implement session cleanup or reporting logic.
pub struct SessionEndHook;

impl Hook for SessionEndHook {
    fn hook_type(&self) -> &str {
        "SessionEnd"
    }

    fn generate_response(&self, _outcomes: &[HandlerOutcome]) -> Value {
        // Return empty object - currently just observing
        // Future: Could generate session summaries or cleanup resources
        json!({})
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_start_hook_response() {
        let hook = SessionStartHook;
        assert_eq!(hook.hook_type(), "SessionStart");

        let response = hook.generate_response(&[]);
        assert_eq!(response, json!({}));
    }

    #[test]
    fn test_session_end_hook_response() {
        let hook = SessionEndHook;
        assert_eq!(hook.hook_type(), "SessionEnd");

        let response = hook.generate_response(&[]);
        assert_eq!(response, json!({}));
    }
}
