# Getting Started with Claudifier

Claudifier is a universal notification handler for Claude Code events. It reads JSON events from stdin and dispatches them to various notification handlers based on your configuration.

## Installation

### Via Homebrew (Recommended)

```bash
brew tap terraboops/claudifier https://github.com/terraboops/claudifier
brew install claudifier
```

### From Source

**Prerequisites:**
- Rust toolchain (install from https://rustup.rs)
- For Signal notifications: signal-cli (https://github.com/AsamK/signal-cli)

```bash
# Clone the repository
git clone https://github.com/terraboops/claudifier.git
cd claudifier

# Build and install
make install
```

## Quick Start

### 1. List Available Handlers

```bash
claudifier --list-handlers
```

This will show all available notification types:
- desktop
- sound
- signal
- webhook
- email

### 2. Create Configuration

Claudifier automatically finds your config file:
1. **Project-specific**: `$CLAUDE_PROJECT_DIR/.claude/claudifier.json` (when run via Claude Code hooks)
2. **Global fallback**: `~/.claude/claudifier.json`

Create a `.claude/claudifier.json` file in your project (or globally at `~/.claude/claudifier.json`):

```json
{
  "handlers": [
    {
      "name": "success-notification",
      "type": "desktop",
      "match_rules": {
        "status": "success"
      },
      "config": {
        "summary": "Build Success",
        "body": "Task {{task}} completed successfully!",
        "urgency": "normal",
        "timeout": 5000
      }
    },
    {
      "name": "random-success-sounds",
      "type": "sound",
      "match_rules": {
        "status": "success"
      },
      "config": {
        "files": [
          "~/sounds/success1.wav",
          "~/sounds/success2.mp3",
          "~/sounds/success3.wav"
        ],
        "random": true,
        "volume": 0.8
      }
    }
  ]
}
```

### 3. Test with Sample Event

```bash
# Process one event and exit
echo '{"status": "success", "task": "build"}' | claudifier

# Show debug output including ALSA warnings
echo '{"status": "success", "task": "build"}' | claudifier --debug
```

## Configuration Guide

### Handler Configuration

Each handler has the following structure:

```json
{
  "name": "unique-handler-name",
  "type": "handler-type",
  "match_rules": { /* optional matching rules */ },
  "config": { /* handler-specific configuration */ }
}
```

### Event Matching

**Simple matching:**
```json
"match_rules": {
  "event_type": "build_complete",
  "status": "success"
}
```

**Complex matching:**
```json
"match_rules": {
  "all": [
    {"event_type": "test"},
    {"status": "success"}
  ],
  "any": [
    {"priority": "high"},
    {"priority": "critical"}
  ],
  "not": {
    "ignored": true
  }
}
```

**Nested fields:**
```json
"match_rules": {
  "tool.name": "bash",
  "tool.exit_code": 0
}
```

**No rules (match all):**
```json
"match_rules": null
```

### Secrets Management

Claudifier supports secure credential management:

**Environment variables:**
```json
{
  "config": {
    "webhook_url": "{{env.SLACK_WEBHOOK_URL}}"
  }
}
```

**File-based secrets:**
```json
{
  "config": {
    "api_key": "{{file.~/.secrets/api-key}}"
  }
}
```

## Handler Types

### Desktop Notifications

```json
{
  "type": "desktop",
  "config": {
    "summary": "Notification Title",
    "body": "Notification body with {{variable}} substitution",
    "urgency": "normal",  // low, normal, critical
    "timeout": 5000       // milliseconds
  }
}
```

### Sound

Play audio files using rodio. Supports WAV, MP3, and other common formats.

**Single sound file:**
```json
{
  "type": "sound",
  "config": {
    "file": "/path/to/sound.wav",
    "volume": 0.8         // 0.0 to 1.0, default 1.0
  }
}
```

**Multiple files with random selection:**
```json
{
  "type": "sound",
  "config": {
    "files": [
      "/path/to/sound1.wav",
      "/path/to/sound2.mp3",
      "/path/to/sound3.wav"
    ],
    "random": true,       // randomly select from files array
    "volume": 1.0
  }
}
```

**Multiple files without random (uses first):**
```json
{
  "type": "sound",
  "config": {
    "files": ["/path/to/primary.wav", "/path/to/fallback.wav"],
    "volume": 0.8
  }
}
```

### Signal

```json
{
  "type": "signal",
  "config": {
    "recipient": "+1234567890",
    "message": "Build {{status}}: {{details}}",
    "signal_cli_path": "signal-cli",  // optional
    "account": "+9876543210"          // optional sender
  }
}
```

### Webhook (Slack, Discord, IFTTT, etc.)

**Slack:**
```json
{
  "type": "webhook",
  "config": {
    "url": "{{env.SLACK_WEBHOOK_URL}}",
    "type": "slack",
    "text": "Build {{status}}",
    "channel": "#builds",
    "username": "Claude Code Bot"
  }
}
```

**Discord:**
```json
{
  "type": "webhook",
  "config": {
    "url": "{{env.DISCORD_WEBHOOK_URL}}",
    "type": "discord",
    "content": "Build {{status}}",
    "username": "Claude Code"
  }
}
```

**Generic JSON:**
```json
{
  "type": "webhook",
  "config": {
    "url": "https://your-server.com/webhook",
    "type": "json",
    "payload": {
      "event": "{{event_type}}",
      "data": "{{details}}"
    }
  }
}
```

### Email

```json
{
  "type": "email",
  "config": {
    "from": "bot@example.com",
    "to": "user@example.com",
    "subject": "Build {{status}}",
    "body": "Details: {{message}}",
    "smtp_server": "smtp.example.com",
    "username": "{{env.SMTP_USER}}",
    "password": "{{env.SMTP_PASS}}"
  }
}
```

## Integration with Claude Code

### Configuring Hooks

Add claudifier to your Claude Code hooks in `~/.claude/settings.json` or `.claude/settings.json`:

```json
{
  "hooks": {
    "Notification": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "claudifier"
          }
        ]
      }
    ],
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "claudifier"
          }
        ]
      }
    ]
  }
}
```

Available hook events include:
- `Notification` - When Claude sends a notification
- `Stop` - When Claude finishes responding
- `PostToolUse` - After a tool is used
- `PermissionRequest` - When permission is requested
- And more (see [Claude Code hooks docs](https://code.claude.com/docs/en/hooks))

### Debug Mode

To troubleshoot, add `--debug` flag to the command:

```json
{
  "type": "command",
  "command": "claudifier --debug"
}
```

This logs to `/tmp/claudifier.log`.

## Common Use Cases

### Build Notifications

Get desktop notifications when builds complete:

```json
{
  "name": "build-complete",
  "type": "desktop",
  "match_rules": {"event_type": "build_complete"},
  "config": {
    "summary": "Build Complete",
    "body": "Exit code: {{exit_code}}"
  }
}
```

### Error Alerts

Send Signal messages on errors:

```json
{
  "name": "error-alert",
  "type": "signal",
  "match_rules": {"severity": "error"},
  "config": {
    "recipient": "+1234567890",
    "message": "Error: {{message}}"
  }
}
```

### Team Notifications

Post to Slack on important events:

```json
{
  "name": "team-update",
  "type": "webhook",
  "match_rules": {"priority": "high"},
  "config": {
    "url": "{{env.SLACK_WEBHOOK}}",
    "type": "slack",
    "text": "High priority event: {{details}}",
    "channel": "#dev-team"
  }
}
```

## Troubleshooting

### signal-cli not found

Install signal-cli:
```bash
# Arch Linux
yay -S signal-cli

# Or from source
# See https://github.com/AsamK/signal-cli
```

### Desktop notifications not working

Ensure your system has a notification daemon running (e.g., dunst, notify-osd).

### Webhook failures

- Check your webhook URL is correct
- Verify environment variables are set
- Use `--debug` to see detailed error messages

### ALSA Warnings on Linux (Homebrew)

When using the Homebrew-installed version on Linux, you may see ALSA warnings like:

```
ALSA lib dlmisc.c:339:(snd_dlobj_cache_get0) Cannot open shared library libasound_module_pcm_pulse.so
ALSA lib pcm_rate.c:1581:(snd_pcm_rate_open) Cannot find rate converter
```

**These warnings are cosmetic and do not affect functionality.** They occur because Homebrew's `alsa-lib` package is missing optional plugin libraries (PulseAudio, JACK, PipeWire, etc.). Sound playback works correctly via direct ALSA.

**To suppress these warnings:**

```bash
# Option 1: Redirect stderr when invoking claudifier
claudifier 2>/dev/null

# Option 2: Set environment variable
LIBASOUND_DEBUG=0 claudifier

# Option 3: In Claude Code hooks config, redirect stderr
{
  "hooks": {
    "Notification": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "sh -c 'claudifier 2>/dev/null'"
          }
        ]
      }
    ]
  }
}
```

Note: Binaries built locally with `cargo install` or `make install` do not show these warnings.

## Further Reading

- See `CLAUDE.md` for development details
- Check `Cargo.toml` for dependency information
- Run `claudifier --help` for CLI options
