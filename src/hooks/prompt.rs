//! UserPromptSubmit hook implementation.

use super::{HandlerOutcome, Hook};
use serde_json::{json, Value};

/// Handler for UserPromptSubmit hooks.
///
/// These hooks fire when the user submits a prompt to Claude Code.
/// Returns an empty object {} to allow passive observation.
/// Future: Could implement prompt transformation or validation logic.
pub struct UserPromptSubmitHook;

impl Hook for UserPromptSubmitHook {
    fn hook_type(&self) -> &str {
        "UserPromptSubmit"
    }

    fn generate_response(&self, _outcomes: &[HandlerOutcome]) -> Value {
        // Return empty object - currently just observing
        // Future: Could modify the prompt or add system messages
        // based on handler outcomes
        json!({})
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_prompt_submit_hook_response() {
        let hook = UserPromptSubmitHook;
        assert_eq!(hook.hook_type(), "UserPromptSubmit");

        let response = hook.generate_response(&[]);
        assert_eq!(response, json!({}));
    }
}
