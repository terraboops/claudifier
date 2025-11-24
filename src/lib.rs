//! Boopifier - A universal notification receiver for Claude Code events.
//!
//! This library receives JSON events from Claude Code hooks via stdin and dispatches
//! them to various notification handlers based on configuration.
//!
//! # Architecture
//!
//! - **Event**: JSON events from Claude Code
//! - **Config**: Configuration from `.claude/boopifier.json`
//! - **Matcher**: Pattern matching to filter events
//! - **Handlers**: Pluggable notification targets (desktop, sound, signal, webhook, email)
//!
//! # Examples
//!
//! Processing events from stdin:
//!
//! ```no_run
//! use boopifier::{Config, HandlerRegistry, process_event};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = Config::load(".claude/boopifier.json")?;
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
///
/// Handlers are executed in parallel for better performance.
pub async fn process_event(
    event_json: &str,
    config: &Config,
    registry: &HandlerRegistry,
) -> anyhow::Result<Vec<HandlerOutcome>> {
    use futures::future::join_all;

    let event = Event::from_json(event_json)?;

    // Collect futures for all matching handlers
    let mut handler_futures = Vec::new();

    for handler_config in &config.handlers {
        // Check if event matches the handler's rules
        if !matches(&event, &handler_config.match_rules, &handler_config.match_type) {
            continue;
        }

        // Get the handler
        let handler = match registry.get(&handler_config.handler_type) {
            Some(h) => h,
            None => {
                return Ok(vec![HandlerOutcome::Error(format!(
                    "{}: Unknown handler type: {}",
                    handler_config.name, handler_config.handler_type
                ))]);
            }
        };

        // Clone data for this handler future
        let event_clone = event.clone();
        let config_clone = handler_config.config.clone();
        let name = handler_config.name.clone();

        // Create a future for this handler
        let future = async move {
            match handler.handle(&event_clone, &config_clone).await {
                Ok(()) => HandlerOutcome::Success,
                Err(e) => HandlerOutcome::Error(format!("{}: {}", name, e)),
            }
        };

        handler_futures.push(future);
    }

    // Execute all handler futures concurrently
    let outcomes = join_all(handler_futures).await;

    Ok(outcomes)
}
