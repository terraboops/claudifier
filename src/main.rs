//! Boopifier - Universal notification receiver for Claude Code events.
//!
//! Reads JSON events from stdin and dispatches them to configured handlers.

use clap::Parser;
use boopifier::{hook_from_event, process_event, Config, Event, HandlerOutcome, HandlerRegistry};
use serde_json::json;
use std::fs::OpenOptions;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(name = "boopifier")]
#[command(author, version, about)]
#[command(about = "Universal notification handler for Claude Code events")]
struct Cli {
    /// Path to the configuration file (overrides auto-detection)
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Enable debug logging to /tmp/boopifier.log
    #[arg(short, long)]
    debug: bool,

    /// List available handler types
    #[arg(long)]
    list_handlers: bool,
}

#[cfg(target_os = "linux")]
fn suppress_alsa_warnings() {
    extern "C" {
        fn snd_lib_error_set_handler(handler: Option<extern "C" fn()>);
    }
    unsafe {
        snd_lib_error_set_handler(None);
    }
}

// Suppress ALSA warnings as early as possible (before main)
#[cfg(target_os = "linux")]
#[used]
#[link_section = ".init_array"]
static ALSA_INIT: extern "C" fn() = {
    extern "C" fn alsa_init() {
        extern "C" {
            fn snd_lib_error_set_handler(handler: Option<extern "C" fn()>);
        }
        unsafe {
            snd_lib_error_set_handler(None);
        }
    }
    alsa_init
};

struct DebugLogger {
    enabled: bool,
    log_path: PathBuf,
}

impl DebugLogger {
    fn new(enabled: bool) -> Self {
        Self {
            enabled,
            log_path: PathBuf::from("/tmp/boopifier.log"),
        }
    }

    fn log(&self, message: &str) {
        if !self.enabled {
            return;
        }

        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
        {
            let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
            let _ = writeln!(file, "[{}] {}", timestamp, message);
        }
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let logger = DebugLogger::new(cli.debug);

    // Set global debug mode for handlers
    boopifier::set_debug_mode(cli.debug);

    // Suppress ALSA errors early (before any audio initialization)
    #[cfg(target_os = "linux")]
    if !cli.debug {
        suppress_alsa_warnings();
    }

    // List handlers if requested
    if cli.list_handlers {
        list_available_handlers();
        return;
    }

    logger.log("Boopifier starting");

    // Resolve config file path
    let config_path = match &cli.config {
        Some(path) => {
            logger.log(&format!("Using config from CLI arg: {:?}", path));
            path.clone()
        }
        None => {
            let resolved = resolve_config_path();
            logger.log(&format!("Auto-detected config: {:?}", resolved));
            resolved
        }
    };

    // Load configuration (secrets are resolved automatically)
    let mut config = match Config::load(&config_path) {
        Ok(cfg) => cfg,
        Err(e) => {
            logger.log(&format!("Failed to load config: {}", e));
            output_hook_error(&format!("Failed to load config from {:?}: {}", config_path, e));
            process::exit(0); // Exit 0 for hook compatibility
        }
    };

    // Apply project-specific overrides if using global config
    if let Ok(project_dir) = std::env::var("CLAUDE_PROJECT_DIR") {
        // Only apply overrides if we're not using a project-specific config
        let project_config_path = PathBuf::from(&project_dir).join(".claude/boopifier.json");
        if !project_config_path.exists() {
            logger.log(&format!("Checking overrides for project: {}", project_dir));
            config.apply_overrides(&project_dir);
        }
    }

    logger.log(&format!("Loaded config with {} handlers", config.handlers.len()));

    // Create handler registry
    let registry = HandlerRegistry::new();

    // Read one event from stdin (Claude Code sends one event per invocation)
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let mut event_json = String::new();

    match reader.read_line(&mut event_json) {
        Ok(_) => {
            if event_json.trim().is_empty() {
                logger.log("No input received");
                println!("{{}}");
                return;
            }

            logger.log(&format!("Received event: {}", event_json.trim()));

            // Parse the event to determine hook type
            let event = match Event::from_json(&event_json) {
                Ok(e) => e,
                Err(e) => {
                    logger.log(&format!("Failed to parse event JSON: {}", e));
                    output_hook_error(&format!("Invalid JSON: {}", e));
                    return;
                }
            };

            // Create the appropriate hook type
            let hook = match hook_from_event(&event) {
                Ok(h) => {
                    logger.log(&format!("Hook type: {}", h.hook_type()));
                    h
                }
                Err(e) => {
                    logger.log(&format!("Unknown hook type: {}", e));
                    output_hook_error(&format!("Unknown hook: {}", e));
                    return;
                }
            };

            // Process the event through handlers
            match process_event(&event_json, &config, &registry).await {
                Ok(outcomes) => {
                    // Log handler outcomes
                    let successes = outcomes.iter().filter(|o| matches!(o, HandlerOutcome::Success)).count();
                    let errors = outcomes.iter().filter(|o| matches!(o, HandlerOutcome::Error(_))).count();

                    if errors == 0 {
                        logger.log(&format!("Event processed successfully ({} handlers)", successes));
                    } else {
                        logger.log(&format!("Event processed: {} succeeded, {} failed", successes, errors));
                        for outcome in &outcomes {
                            if let HandlerOutcome::Error(msg) = outcome {
                                logger.log(&format!("Handler error: {}", msg));
                            }
                        }
                    }

                    // Generate hook-specific response
                    let response = hook.generate_response(&outcomes);
                    if let Ok(json_str) = serde_json::to_string(&response) {
                        println!("{}", json_str);
                    }
                }
                Err(e) => {
                    logger.log(&format!("Error processing event: {}", e));
                    // Still output a valid response (empty object)
                    println!("{{}}");
                }
            }

            logger.log("Event processed, exiting");
        }
        Err(e) => {
            logger.log(&format!("Error reading stdin: {}", e));
            output_hook_error(&format!("Error reading stdin: {}", e));
        }
    }

    // Explicitly exit to avoid hanging on background threads (rodio/tokio cleanup)
    process::exit(0);
}

/// Resolve the config file path using Claude Code conventions.
///
/// Resolution order:
/// 1. $CLAUDE_PROJECT_DIR/.claude/boopifier.json (if CLAUDE_PROJECT_DIR is set and file exists)
/// 2. ~/.claude/boopifier.json (global fallback, may include path-based overrides)
///
/// Note: When using the global config, project-specific overrides will be applied
/// based on glob pattern matching against $CLAUDE_PROJECT_DIR.
fn resolve_config_path() -> PathBuf {
    // Try project-specific config if CLAUDE_PROJECT_DIR is set
    if let Ok(project_dir) = std::env::var("CLAUDE_PROJECT_DIR") {
        let project_config = PathBuf::from(project_dir).join(".claude/boopifier.json");
        if project_config.exists() {
            return project_config;
        }
    }

    // Fall back to global config
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".claude/boopifier.json")
}

fn list_available_handlers() {
    let registry = HandlerRegistry::new();
    println!("Available notification handlers:");
    for handler_type in registry.list_types() {
        println!("  - {}", handler_type);
    }
}

/// Output error hook response in Claude Code format (still continues)
fn output_hook_error(error_message: &str) {
    let response = json!({
        "continue": true,
        "systemMessage": format!("Boopifier warning: {}", error_message)
    });

    if let Ok(json_str) = serde_json::to_string(&response) {
        println!("{}", json_str);
    }
}
