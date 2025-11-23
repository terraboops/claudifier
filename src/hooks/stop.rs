//! Stop and SubagentStop hook implementation.

use super::{HandlerOutcome, Hook};
use serde_json::{json, Value};

/// Handler for Stop and SubagentStop hooks.
///
/// These hooks fire when Claude Code is about to stop execution.
/// We return an empty object {} to indicate passive observation (no decision).
pub struct StopHook {
    hook_name: String,
}

impl StopHook {
    pub fn new(hook_name: &str) -> Self {
        Self {
            hook_name: hook_name.to_string(),
        }
    }
}

impl Hook for StopHook {
    fn hook_type(&self) -> &str {
        &self.hook_name
    }

    fn generate_response(&self, _outcomes: &[HandlerOutcome]) -> Value {
        // Return empty object - we're just observing events, not controlling behavior
        // To block stopping, we would return:
        // {"decision": "block", "reason": "...", "continue": false}
        json!({})
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stop_hook_response() {
        let hook = StopHook::new("Stop");
        assert_eq!(hook.hook_type(), "Stop");

        let response = hook.generate_response(&[]);
        assert_eq!(response, json!({}));
    }

    #[test]
    fn test_subagent_stop_hook_response() {
        let hook = StopHook::new("SubagentStop");
        assert_eq!(hook.hook_type(), "SubagentStop");

        let response = hook.generate_response(&[]);
        assert_eq!(response, json!({}));
    }
}
