//! PreCompact hook implementation.

use super::{HandlerOutcome, Hook};
use serde_json::{json, Value};

/// Handler for PreCompact hooks.
///
/// These hooks fire before Claude Code compacts conversation history.
/// Returns an empty object {} to allow passive observation.
/// Future: Could implement logic to prevent compaction or save history.
pub struct PreCompactHook;

impl Hook for PreCompactHook {
    fn hook_type(&self) -> &str {
        "PreCompact"
    }

    fn generate_response(&self, _outcomes: &[HandlerOutcome]) -> Value {
        // Return empty object - currently just observing
        // Future: Could return {"decision": "block"} to prevent compaction
        // or archive conversation history before it's compacted
        json!({})
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pre_compact_hook_response() {
        let hook = PreCompactHook;
        assert_eq!(hook.hook_type(), "PreCompact");

        let response = hook.generate_response(&[]);
        assert_eq!(response, json!({}));
    }
}
