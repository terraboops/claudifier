//! Configuration system for claudifier.
//!
//! This module handles loading and parsing `.claude/claudifier.json` configuration files.

use crate::error::{NotificationError, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::Path;

/// Project-specific override configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectOverride {
    /// Glob pattern to match project paths (e.g., "/home/user/work/*")
    pub path_pattern: String,

    /// Handlers to use when this pattern matches (replaces base handlers)
    pub handlers: Vec<HandlerConfig>,
}

/// Main configuration structure.
#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    /// List of notification handlers to configure
    pub handlers: Vec<HandlerConfig>,

    /// Optional project-specific overrides based on path patterns
    #[serde(default)]
    pub overrides: Option<Vec<ProjectOverride>>,
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("handler_count", &self.handlers.len())
            .field("handlers", &self.handlers)
            .finish()
    }
}

/// Match type for string matching
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MatchType {
    /// Exact string match (default)
    Exact,
    /// Regular expression match
    Regex,
}

impl Default for MatchType {
    fn default() -> Self {
        MatchType::Exact
    }
}

/// Configuration for a single notification handler.
#[derive(Clone, Serialize, Deserialize)]
pub struct HandlerConfig {
    /// Unique name for this handler instance
    pub name: String,

    /// Handler type (desktop, sound, signal, webhook, email, etc.)
    #[serde(rename = "type")]
    pub handler_type: String,

    /// Event matching criteria (optional - if None, matches all events)
    #[serde(default)]
    pub match_rules: Option<MatchRules>,

    /// Match type for string fields (defaults to "exact")
    #[serde(default)]
    pub match_type: MatchType,

    /// Handler-specific configuration
    pub config: HashMap<String, Value>,
}

impl fmt::Debug for HandlerConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HandlerConfig")
            .field("name", &self.name)
            .field("type", &self.handler_type)
            .field("match_rules", &self.match_rules)
            .finish_non_exhaustive()
    }
}

/// Rules for matching events.
///
/// # Serialization
///
/// Simple rules are just a map of field names to values:
/// ```json
/// {"hook_event_name": "Notification"}
/// ```
///
/// Complex rules must have at least one of: "all", "any", or "not":
/// ```json
/// {"any": [{"hook_event_name": "Notification"}, {"hook_event_name": "Stop"}]}
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MatchRules {
    /// Simple key-value matching
    Simple(HashMap<String, Value>),

    /// Complex matching with operators ("all", "any", "not")
    Complex {
        all: Option<Vec<HashMap<String, Value>>>,
        any: Option<Vec<HashMap<String, Value>>>,
        not: Option<HashMap<String, Value>>,
    },
}

impl Config {
    /// Loads configuration from a file path and resolves secrets.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read, parsed, or secrets cannot be resolved.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path.as_ref()).map_err(|e| {
            NotificationError::InvalidConfig(format!("Failed to read config file: {}", e))
        })?;

        let mut config = Self::from_json(&content)?;
        config.resolve_secrets()?;
        Ok(config)
    }

    /// Parses configuration from a JSON string.
    ///
    /// # Errors
    ///
    /// Returns an error if the JSON is invalid.
    pub fn from_json(json: &str) -> Result<Self> {
        let config: Config = serde_json::from_str(json).map_err(|e| {
            NotificationError::InvalidConfig(format!("Invalid JSON: {}", e))
        })?;

        Ok(config)
    }

    /// Applies project-specific overrides based on the current project path.
    ///
    /// If multiple patterns match, the last match wins. If a pattern matches,
    /// the override handlers completely replace the base handlers.
    ///
    /// # Arguments
    ///
    /// * `project_path` - The current project directory path (usually from $CLAUDE_PROJECT_DIR)
    pub fn apply_overrides(&mut self, project_path: &str) {
        let Some(overrides) = &self.overrides else {
            return;
        };

        // Find the last matching override
        let mut last_match: Option<&ProjectOverride> = None;
        for override_config in overrides {
            if glob::Pattern::new(&override_config.path_pattern)
                .ok()
                .and_then(|pattern| Some(pattern.matches(project_path)))
                .unwrap_or(false)
            {
                last_match = Some(override_config);
            }
        }

        // Apply the last matching override
        if let Some(matched_override) = last_match {
            self.handlers = matched_override.handlers.clone();
        }
    }

    /// Resolves secrets in configuration values.
    ///
    /// Supports:
    /// - `{{env.VAR_NAME}}` - Environment variables
    /// - `{{file.path/to/file}}` - Read from file
    /// - `{{keychain.service/key}}` - System keychain (future)
    ///
    /// Note: This is called automatically by `load()`. Only use this directly
    /// when working with configurations created via `from_json()`.
    fn resolve_secrets(&mut self) -> Result<()> {
        // Resolve secrets in main handlers
        for handler in &mut self.handlers {
            for value in handler.config.values_mut() {
                if let Some(s) = value.as_str() {
                    let resolved = Self::resolve_secret_string(s)?;
                    *value = Value::String(resolved);
                }
            }
        }

        // Also resolve secrets in override handlers
        if let Some(overrides) = &mut self.overrides {
            for override_config in overrides {
                for handler in &mut override_config.handlers {
                    for value in handler.config.values_mut() {
                        if let Some(s) = value.as_str() {
                            let resolved = Self::resolve_secret_string(s)?;
                            *value = Value::String(resolved);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn resolve_secret_string(s: &str) -> Result<String> {
        let mut result = s.to_string();

        // Environment variables: {{env.VAR_NAME}}
        if let Some(start) = result.find("{{env.") {
            if let Some(end) = result[start..].find("}}") {
                let var_name = &result[start + 6..start + end];
                let value = std::env::var(var_name).map_err(|_| {
                    NotificationError::InvalidConfig(format!(
                        "Environment variable not found: {}",
                        var_name
                    ))
                })?;
                result = result.replace(&format!("{{{{env.{}}}}}", var_name), &value);
            }
        }

        // File: {{file.path/to/file}}
        if let Some(start) = result.find("{{file.") {
            if let Some(end) = result[start..].find("}}") {
                let file_path = &result[start + 7..start + end];
                let expanded_path = shellexpand::tilde(file_path);
                let value = fs::read_to_string(expanded_path.as_ref())
                    .map_err(|e| {
                        NotificationError::InvalidConfig(format!(
                            "Failed to read file {}: {}",
                            file_path, e
                        ))
                    })?
                    .trim()
                    .to_string();
                result = result.replace(&format!("{{{{file.{}}}}}", file_path), &value);
            }
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config() {
        let json = r#"{
            "handlers": [
                {
                    "name": "test-handler",
                    "type": "desktop",
                    "match": {"event_type": "success"},
                    "config": {"timeout": 5000}
                }
            ]
        }"#;

        let config = Config::from_json(json).unwrap();
        assert_eq!(config.handlers.len(), 1);
        assert_eq!(config.handlers[0].name, "test-handler");
        assert_eq!(config.handlers[0].handler_type, "desktop");
    }

    #[test]
    fn test_env_var_resolution() {
        std::env::set_var("TEST_VAR", "secret_value");
        let resolved = Config::resolve_secret_string("prefix_{{env.TEST_VAR}}_suffix").unwrap();
        assert_eq!(resolved, "prefix_secret_value_suffix");
    }

    #[test]
    fn test_override_exact_match() {
        let json = r#"{
            "handlers": [{"name": "base", "type": "desktop", "config": {}}],
            "overrides": [
                {
                    "path_pattern": "/home/user/work/project1",
                    "handlers": [{"name": "work", "type": "sound", "config": {}}]
                }
            ]
        }"#;

        let mut config = Config::from_json(json).unwrap();
        assert_eq!(config.handlers[0].name, "base");

        config.apply_overrides("/home/user/work/project1");
        assert_eq!(config.handlers.len(), 1);
        assert_eq!(config.handlers[0].name, "work");
    }

    #[test]
    fn test_override_glob_wildcard() {
        let json = r#"{
            "handlers": [{"name": "base", "type": "desktop", "config": {}}],
            "overrides": [
                {
                    "path_pattern": "/home/user/work/*",
                    "handlers": [{"name": "work", "type": "sound", "config": {}}]
                }
            ]
        }"#;

        let mut config = Config::from_json(json).unwrap();
        config.apply_overrides("/home/user/work/project1");
        assert_eq!(config.handlers[0].name, "work");

        let mut config2 = Config::from_json(json).unwrap();
        config2.apply_overrides("/home/user/work/project2");
        assert_eq!(config2.handlers[0].name, "work");
    }

    #[test]
    fn test_override_last_match_wins() {
        let json = r#"{
            "handlers": [{"name": "base", "type": "desktop", "config": {}}],
            "overrides": [
                {
                    "path_pattern": "/home/user/work/*",
                    "handlers": [{"name": "work-general", "type": "sound", "config": {}}]
                },
                {
                    "path_pattern": "/home/user/work/special",
                    "handlers": [{"name": "work-special", "type": "webhook", "config": {}}]
                }
            ]
        }"#;

        let mut config = Config::from_json(json).unwrap();
        config.apply_overrides("/home/user/work/special");
        assert_eq!(config.handlers[0].name, "work-special");
    }

    #[test]
    fn test_override_no_match() {
        let json = r#"{
            "handlers": [{"name": "base", "type": "desktop", "config": {}}],
            "overrides": [
                {
                    "path_pattern": "/home/user/work/*",
                    "handlers": [{"name": "work", "type": "sound", "config": {}}]
                }
            ]
        }"#;

        let mut config = Config::from_json(json).unwrap();
        config.apply_overrides("/home/user/personal/project");
        assert_eq!(config.handlers[0].name, "base");
    }

    #[test]
    fn test_override_no_overrides_field() {
        let json = r#"{
            "handlers": [{"name": "base", "type": "desktop", "config": {}}]
        }"#;

        let mut config = Config::from_json(json).unwrap();
        config.apply_overrides("/any/path");
        assert_eq!(config.handlers[0].name, "base");
    }
}
