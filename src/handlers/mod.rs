//! Notification handlers.
//!
//! This module defines the handler trait and provides implementations for various
//! notification targets.

use crate::error::Result;
use crate::event::Event;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

pub mod desktop;
pub mod email;
pub mod signal;
pub mod sound;
pub mod webhook;

/// Error type for handler operations.
pub type HandlerResult<T> = Result<T>;

/// Trait for notification handlers.
#[async_trait]
pub trait Handler: Send + Sync {
    /// Returns the handler type name.
    fn handler_type(&self) -> &str;

    /// Handles an event notification.
    ///
    /// # Arguments
    ///
    /// * `event` - The event that triggered this handler
    /// * `config` - Handler-specific configuration from claudifier.json
    async fn handle(&self, event: &Event, config: &HashMap<String, Value>) -> HandlerResult<()>;
}

/// Registry for managing notification handlers.
pub struct HandlerRegistry {
    handlers: HashMap<String, Box<dyn Handler>>,
}

impl HandlerRegistry {
    /// Creates a new handler registry with all built-in handlers.
    pub fn new() -> Self {
        let mut registry = Self {
            handlers: HashMap::new(),
        };

        // Register all built-in handlers
        registry.register(Box::new(desktop::DesktopHandler));
        registry.register(Box::new(sound::SoundHandler));
        registry.register(Box::new(signal::SignalHandler));
        registry.register(Box::new(webhook::WebhookHandler));
        registry.register(Box::new(email::EmailHandler));

        registry
    }

    /// Registers a new handler.
    pub fn register(&mut self, handler: Box<dyn Handler>) {
        let handler_type = handler.handler_type().to_string();
        self.handlers.insert(handler_type, handler);
    }

    /// Gets a handler by type.
    pub fn get(&self, handler_type: &str) -> Option<&dyn Handler> {
        self.handlers.get(handler_type).map(|b| b.as_ref())
    }

    /// Lists all registered handler types.
    pub fn list_types(&self) -> Vec<&str> {
        self.handlers.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for HandlerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_contains_handlers() {
        let registry = HandlerRegistry::new();
        assert!(registry.get("desktop").is_some());
        assert!(registry.get("sound").is_some());
        assert!(registry.get("signal").is_some());
        assert!(registry.get("webhook").is_some());
        assert!(registry.get("email").is_some());
    }

    #[test]
    fn test_list_types() {
        let registry = HandlerRegistry::new();
        let types = registry.list_types();
        assert!(types.contains(&"desktop"));
        assert!(types.contains(&"sound"));
    }
}
