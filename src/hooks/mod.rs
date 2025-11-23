//! Hook type system for Claude Code hooks.
//!
//! Each hook type (Stop, Notification, PreToolUse, etc.) has specific response requirements.
//! This module provides a trait-based abstraction for handling different hook types.

pub mod stop;
pub mod notification;
pub mod tool_use;
pub mod permission;
pub mod prompt;
pub mod session;
pub mod compact;

use crate::event::Event;
use anyhow::{bail, Result};
use serde_json::Value;

/// Outcome from executing a notification handler
#[derive(Debug, Clone)]
pub enum HandlerOutcome {
    /// Handler succeeded
    Success,
    /// Handler failed with an error
    Error(String),
    /// Handler requires user interaction (future: for PreToolUse)
    #[allow(dead_code)]
    Interactive(InteractiveResponse),
}

/// Interactive response from a handler (for PreToolUse hooks)
#[derive(Debug, Clone)]
pub struct InteractiveResponse {
    pub decision: PermissionDecision,
    pub reason: Option<String>,
}

/// Permission decision for tool use
#[derive(Debug, Clone)]
pub enum PermissionDecision {
    Allow,
    Deny,
    Ask,
}

/// Trait for Claude Code hook types.
///
/// Each hook type knows how to generate its own JSON response format.
pub trait Hook: Send + Sync {
    /// Get the hook type name (e.g., "Stop", "PreToolUse")
    fn hook_type(&self) -> &str;

    /// Generate the appropriate JSON response based on handler outcomes
    fn generate_response(&self, outcomes: &[HandlerOutcome]) -> Value;
}

/// Create a Hook instance from an event by parsing the hook_event_name
pub fn hook_from_event(event: &Event) -> Result<Box<dyn Hook>> {
    let hook_event_name = event
        .get_str("hook_event_name")
        .unwrap_or("unknown");

    match hook_event_name {
        "Stop" | "SubagentStop" => Ok(Box::new(stop::StopHook::new(hook_event_name))),
        "Notification" => Ok(Box::new(notification::NotificationHook)),
        "PreToolUse" => Ok(Box::new(tool_use::PreToolUseHook::from_event(event)?)),
        "PostToolUse" => Ok(Box::new(tool_use::PostToolUseHook)),
        "PermissionRequest" => Ok(Box::new(permission::PermissionRequestHook)),
        "UserPromptSubmit" => Ok(Box::new(prompt::UserPromptSubmitHook)),
        "SessionStart" => Ok(Box::new(session::SessionStartHook)),
        "SessionEnd" => Ok(Box::new(session::SessionEndHook)),
        "PreCompact" => Ok(Box::new(compact::PreCompactHook)),
        _ => bail!("Unknown hook type: {}", hook_event_name),
    }
}
