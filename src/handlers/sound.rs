//! Sound notification handler.
//!
//! Plays audio files using rodio.

use crate::error::NotificationError;
use crate::event::Event;
use crate::handlers::{Handler, HandlerResult};
use async_trait::async_trait;
use rand::seq::SliceRandom;
use rodio::{Decoder, OutputStream, Sink};
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::time::Duration;

// Suppress ALSA warnings on Linux (unless debug mode is enabled)
#[cfg(target_os = "linux")]
fn suppress_alsa_errors_if_not_debug() {
    // Don't suppress if debug mode is enabled - let ALSA warnings show for debugging
    if crate::is_debug_mode() {
        return;
    }

    use std::sync::Once;

    static INIT: Once = Once::new();

    INIT.call_once(|| {
        extern "C" {
            fn snd_lib_error_set_handler(handler: Option<extern "C" fn()>);
        }

        // Set a no-op error handler to suppress ALSA plugin warnings
        unsafe {
            snd_lib_error_set_handler(None);
        }
    });
}

#[cfg(not(target_os = "linux"))]
fn suppress_alsa_errors_if_not_debug() {
    // No-op on non-Linux platforms
}

/// Handler for sound notifications.
pub struct SoundHandler;

#[async_trait]
impl Handler for SoundHandler {
    fn handler_type(&self) -> &str {
        "sound"
    }

    async fn handle(&self, _event: &Event, config: &HashMap<String, Value>) -> HandlerResult<()> {
        // Determine which file to play
        let file_path = get_sound_file(config)?;

        // Expand tilde in path
        let expanded_path = shellexpand::tilde(&file_path);

        // Get optional volume (0.0 to 1.0, default 1.0)
        let volume = config
            .get("volume")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0) as f32;

        // Play the sound
        play_sound(&expanded_path, volume)?;

        Ok(())
    }
}

/// Gets the sound file to play from config.
///
/// Supports:
/// - Single file: `"file": "path/to/sound.wav"`
/// - Multiple files: `"files": ["sound1.wav", "sound2.wav"]`
/// - Random selection: `"random": true` (picks randomly from files array)
fn get_sound_file(config: &HashMap<String, Value>) -> HandlerResult<String> {
    // Check for single file
    if let Some(file) = config.get("file").and_then(|v| v.as_str()) {
        return Ok(file.to_string());
    }

    // Check for files array
    if let Some(files_value) = config.get("files") {
        let files: Vec<String> = match files_value {
            Value::Array(arr) => arr
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect(),
            _ => {
                return Err(NotificationError::InvalidConfig(
                    "Sound handler 'files' must be an array of strings".to_string()
                ))
            }
        };

        if files.is_empty() {
            return Err(NotificationError::InvalidConfig("Sound handler 'files' array is empty".to_string()));
        }

        // Check if random selection is enabled
        let random = config
            .get("random")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if random {
            // Randomly select one file
            let mut rng = rand::thread_rng();
            files
                .choose(&mut rng)
                .cloned()
                .ok_or_else(|| NotificationError::Audio("Failed to randomly select sound file".to_string()))
        } else {
            // Use first file if random not enabled
            Ok(files[0].clone())
        }
    } else {
        Err(NotificationError::InvalidConfig(
            "Sound handler requires either 'file' or 'files' configuration".to_string()
        ))
    }
}

fn play_sound(file_path: &str, volume: f32) -> HandlerResult<()> {
    // Suppress verbose ALSA plugin warnings on Linux (unless debug mode is enabled)
    suppress_alsa_errors_if_not_debug();

    // Get output stream and stream handle
    let (_stream, stream_handle) = OutputStream::try_default()
        .map_err(|e| NotificationError::Audio(format!("Failed to get audio output stream: {}", e)))?;

    // Create a sink for audio playback
    let sink = Sink::try_new(&stream_handle)
        .map_err(|e| NotificationError::Audio(format!("Failed to create audio sink: {}", e)))?;

    // Open the audio file
    let file = File::open(file_path)
        .map_err(|e| NotificationError::Audio(format!("Failed to open audio file '{}': {}", file_path, e)))?;

    let source = Decoder::new(BufReader::new(file))
        .map_err(|e| NotificationError::Audio(format!("Failed to decode audio file: {}", e)))?;

    // Set volume and append to sink
    sink.set_volume(volume.clamp(0.0, 1.0));
    sink.append(source);

    // Wait for sound to finish with a timeout (max 5 seconds)
    // This prevents hanging if there are audio device issues
    let timeout = Duration::from_secs(5);
    let start = std::time::Instant::now();

    while !sink.empty() && start.elapsed() < timeout {
        std::thread::sleep(Duration::from_millis(100));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handler_type() {
        let handler = SoundHandler;
        assert_eq!(handler.handler_type(), "sound");
    }

    #[test]
    fn test_missing_file_config() {
        let handler = SoundHandler;
        let event = Event::from_json(r#"{"test": "data"}"#).unwrap();
        let config = HashMap::new();

        let runtime = tokio::runtime::Runtime::new().unwrap();
        let result = runtime.block_on(handler.handle(&event, &config));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("requires either 'file' or 'files'"));
    }

    #[test]
    fn test_get_sound_file_single() {
        let mut config = HashMap::new();
        config.insert("file".to_string(), Value::String("/path/to/sound.wav".to_string()));

        let result = get_sound_file(&config).unwrap();
        assert_eq!(result, "/path/to/sound.wav");
    }

    #[test]
    fn test_get_sound_file_array_no_random() {
        let mut config = HashMap::new();
        config.insert(
            "files".to_string(),
            Value::Array(vec![
                Value::String("sound1.wav".to_string()),
                Value::String("sound2.wav".to_string()),
            ]),
        );

        let result = get_sound_file(&config).unwrap();
        // Without random, should return first file
        assert_eq!(result, "sound1.wav");
    }

    #[test]
    fn test_get_sound_file_array_with_random() {
        let mut config = HashMap::new();
        config.insert(
            "files".to_string(),
            Value::Array(vec![
                Value::String("sound1.wav".to_string()),
                Value::String("sound2.wav".to_string()),
                Value::String("sound3.wav".to_string()),
            ]),
        );
        config.insert("random".to_string(), Value::Bool(true));

        // Test multiple times to ensure it returns one of the files
        for _ in 0..10 {
            let result = get_sound_file(&config).unwrap();
            assert!(
                result == "sound1.wav" || result == "sound2.wav" || result == "sound3.wav",
                "Got unexpected file: {}",
                result
            );
        }
    }

    #[test]
    fn test_get_sound_file_empty_array() {
        let mut config = HashMap::new();
        config.insert("files".to_string(), Value::Array(vec![]));

        let result = get_sound_file(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }
}
