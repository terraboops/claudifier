//! PreToolUse and PostToolUse hook implementations.

use super::{HandlerOutcome, Hook, PermissionDecision};
use crate::event::Event;
use anyhow::Result;
use serde_json::{json, Value};

/// Handler for PreToolUse hooks.
///
/// These hooks fire before a tool is executed, allowing for approval/denial.
/// Currently auto-approves all tools (passive mode), but can be extended
/// to support interactive handlers that ask the user for permission.
pub struct PreToolUseHook {
    #[allow(dead_code)]
    tool_name: String,
}

impl PreToolUseHook {
    pub fn from_event(event: &Event) -> Result<Self> {
        let tool_name = event
            .get_str("tool_name")
            .unwrap_or("unknown")
            .to_string();

        Ok(Self { tool_name })
    }
}

impl Hook for PreToolUseHook {
    fn hook_type(&self) -> &str {
        "PreToolUse"
    }

    fn generate_response(&self, outcomes: &[HandlerOutcome]) -> Value {
        // Check if any handler provided an interactive response
        let interactive = outcomes.iter().find_map(|outcome| {
            if let HandlerOutcome::Interactive(response) = outcome {
                Some(response)
            } else {
                None
            }
        });

        let (decision, reason) = if let Some(response) = interactive {
            // Use the interactive handler's decision
            let decision_str = match response.decision {
                PermissionDecision::Allow => "allow",
                PermissionDecision::Deny => "deny",
                PermissionDecision::Ask => "ask",
            };
            (decision_str, response.reason.clone())
        } else {
            // Default: auto-approve (passive mode)
            ("allow", None)
        };

        let mut response = json!({
            "hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "permissionDecision": decision
            }
        });

        // Add reason if provided
        if let Some(reason_text) = reason {
            response["hookSpecificOutput"]["permissionDecisionReason"] = json!(reason_text);
        }

        response
    }
}

/// Handler for PostToolUse hooks.
///
/// These hooks fire after a tool has been executed.
/// We return an empty object {} as we're just observing.
pub struct PostToolUseHook;

impl Hook for PostToolUseHook {
    fn hook_type(&self) -> &str {
        "PostToolUse"
    }

    fn generate_response(&self, _outcomes: &[HandlerOutcome]) -> Value {
        // Return empty object - just observing tool execution
        json!({})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::Event;

    #[test]
    fn test_pre_tool_use_auto_approve() {
        let event = Event::from_json(r#"{"hook_event_name": "PreToolUse", "tool_name": "Bash"}"#)
            .unwrap();
        let hook = PreToolUseHook::from_event(&event).unwrap();

        assert_eq!(hook.hook_type(), "PreToolUse");

        let response = hook.generate_response(&[]);
        assert_eq!(
            response,
            json!({
                "hookSpecificOutput": {
                    "hookEventName": "PreToolUse",
                    "permissionDecision": "allow"
                }
            })
        );
    }

    #[test]
    fn test_pre_tool_use_interactive_deny() {
        use crate::hooks::{InteractiveResponse, PermissionDecision};

        let event = Event::from_json(r#"{"hook_event_name": "PreToolUse", "tool_name": "Bash"}"#)
            .unwrap();
        let hook = PreToolUseHook::from_event(&event).unwrap();

        let interactive = InteractiveResponse {
            decision: PermissionDecision::Deny,
            reason: Some("User denied".to_string()),
        };

        let response = hook.generate_response(&[HandlerOutcome::Interactive(interactive)]);
        assert_eq!(
            response,
            json!({
                "hookSpecificOutput": {
                    "hookEventName": "PreToolUse",
                    "permissionDecision": "deny",
                    "permissionDecisionReason": "User denied"
                }
            })
        );
    }

    #[test]
    fn test_post_tool_use_response() {
        let hook = PostToolUseHook;
        assert_eq!(hook.hook_type(), "PostToolUse");

        let response = hook.generate_response(&[]);
        assert_eq!(response, json!({}));
    }
}
