//! PermissionRequest hook implementation.

use super::{HandlerOutcome, Hook};
use serde_json::{json, Value};

/// Handler for PermissionRequest hooks.
///
/// These hooks fire when Claude Code asks for permission to perform an action.
/// Currently returns an empty object {} to allow passive observation.
/// Future: Could implement automatic approval logic based on handler outcomes.
pub struct PermissionRequestHook;

impl Hook for PermissionRequestHook {
    fn hook_type(&self) -> &str {
        "PermissionRequest"
    }

    fn generate_response(&self, _outcomes: &[HandlerOutcome]) -> Value {
        // Return empty object - currently just observing
        // Future: Could return {"decision": "allow"} or {"decision": "deny"}
        // based on handler outcomes to enable automated permission workflows
        json!({})
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_request_hook_response() {
        let hook = PermissionRequestHook;
        assert_eq!(hook.hook_type(), "PermissionRequest");

        let response = hook.generate_response(&[]);
        assert_eq!(response, json!({}));
    }
}
