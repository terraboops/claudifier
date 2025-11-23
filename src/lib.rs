//! Claudifier - A universal notification receiver for Claude Code events.
//!
//! This library receives JSON events from Claude Code hooks via stdin and dispatches
//! them to various notification handlers based on configuration.
//!
//! # Architecture
//!
//! - **Event**: JSON events from Claude Code
//! - **Config**: Configuration from `.claude/claudifier.json`
//! - **Matcher**: Pattern matching to filter events
//! - **Handlers**: Pluggable notification targets (desktop, sound, signal, webhook, email)
//!
//! # Examples
//!
//! Processing events from stdin:
//!
//! ```no_run
//! use claudifier::{Config, HandlerRegistry, process_event};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = Config::load(".claude/claudifier.json")?;
//!     let registry = HandlerRegistry::new();
//!
//!     let event_json = r#"{"event_type": "success", "tool": "bash"}"#;
//!     process_event(event_json, &config, &registry).await?;
//!
//!     Ok(())
//! }
//! ```

pub mod config;
pub mod error;
pub mod event;
pub mod handlers;
pub mod hooks;
pub mod matcher;

use std::sync::atomic::{AtomicBool, Ordering};

/// Global debug mode flag - set once at startup, read from handlers
static DEBUG_MODE: AtomicBool = AtomicBool::new(false);

/// Set global debug mode (call once at startup)
pub fn set_debug_mode(enabled: bool) {
    DEBUG_MODE.store(enabled, Ordering::Relaxed);
}

/// Check if debug mode is enabled
pub fn is_debug_mode() -> bool {
    DEBUG_MODE.load(Ordering::Relaxed)
}

// Re-export commonly used types at the crate root
pub use config::{Config, HandlerConfig, MatchRules, MatchType, ProjectOverride};
pub use error::{NotificationError, Result};
pub use event::Event;
pub use handlers::HandlerRegistry;
pub use hooks::{hook_from_event, HandlerOutcome, Hook};
pub use matcher::matches;

/// Processes a single event through the configured handlers.
///
/// # Errors
///
/// Returns an error if event parsing fails. Handler execution outcomes are collected
/// and returned as a list, allowing all handlers to run even if some fail.
pub async fn process_event(
    event_json: &str,
    config: &Config,
    registry: &HandlerRegistry,
) -> anyhow::Result<Vec<HandlerOutcome>> {
    let event = Event::from_json(event_json)?;
    let mut outcomes = Vec::new();

    for handler_config in &config.handlers {
        // Check if event matches the handler's rules
        if !matches(&event, &handler_config.match_rules, &handler_config.match_type) {
            continue;
        }

        // Get the handler
        let handler = match registry.get(&handler_config.handler_type) {
            Some(h) => h,
            None => {
                outcomes.push(HandlerOutcome::Error(format!(
                    "{}: Unknown handler type: {}",
                    handler_config.name, handler_config.handler_type
                )));
                continue;
            }
        };

        // Execute the handler (collect outcomes)
        match handler.handle(&event, &handler_config.config).await {
            Ok(()) => outcomes.push(HandlerOutcome::Success),
            Err(e) => outcomes.push(HandlerOutcome::Error(format!(
                "{}: {}",
                handler_config.name, e
            ))),
        }
    }

    Ok(outcomes)
}
