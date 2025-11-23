//! Error types for claudifier.
//!
//! This module defines all error types that can occur when using the notification system.

use thiserror::Error;

/// Errors that can occur during notification operations.
#[derive(Error, Debug)]
pub enum NotificationError {
    /// Failed to send a notification to the system.
    #[error("Failed to send notification: {0}")]
    SendFailed(String),

    /// Failed to parse notification configuration.
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// System notification service is unavailable.
    #[error("Notification service unavailable")]
    ServiceUnavailable,

    /// Generic I/O error occurred.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Template rendering error.
    #[error("Template rendering error: {0}")]
    Template(String),

    /// Network request failed (for webhooks).
    #[error("Network request failed: {0}")]
    Network(String),

    /// Email sending error.
    #[error("Email error: {0}")]
    Email(String),

    /// Audio playback error.
    #[error("Audio playback error: {0}")]
    Audio(String),

    /// Handler execution error.
    #[error("Handler error: {0}")]
    Handler(String),
}

/// Result type alias for notification operations.
pub type Result<T> = std::result::Result<T, NotificationError>;
