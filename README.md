# Perch

<p align="center">
  <img src="src-tauri/icons/icon.png" width="128" alt="Perch icon" />
</p>

A mascot/pet companion that watches [opencode](https://github.com/anomalyco/opencode) coding-agent sessions and notifies you when opencode finishes working and goes idle again.

## Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                        Perch Application                         │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────────────┐    ┌──────────────┐    ┌──────────────┐   │
│  │  Plugin Bridge  │───▶│State Machine │───▶│  Notifier    │   │
│  │  (HTTP ingest)  │    │  (Rust)      │    │  (Tauri)     │   │
│  └────────┬────────┘    └──────────────┘    └──────────────┘   │
│           │                       │                    │         │
│           │    ┌─────────────┐    │                    │         │
│           └───▶│  SSE Client │────┘                    │         │
│                │  (fallback) │                         │         │
│                └─────────────┘                         │         │
│                       │                                │         │
│                       ▼                                ▼         │
│                ┌──────────────┐                ┌──────────────┐ │
│                │    Tray      │                │   Native     │ │
│                │    Icon      │                │ Notification │ │
│                └──────────────┘                └──────────────┘ │
│                                                                  │
├──────────────────────────────────────────────────────────────────┤
│                      Future Modules (Stubbed)                    │
│  ┌──────────────┐  ┌──────────────┐                              │
│  │   Overlay    │  │   Settings   │                              │
│  │   Renderer   │  │    Store     │                              │
│  └──────────────┘  └──────────────┘                              │
└──────────────────────────────────────────────────────────────────┘
```

### Event Sources

Perch supports two event source modes:

| Mode | Default | Description |
|------|---------|-------------|
| **Plugin Bridge** | ✅ Yes | opencode plugin forwards events to Perch's local HTTP server |
| **SSE Client** | No (opt-in) | Perch connects directly to opencode's SSE stream |

**Plugin Bridge** is the default because it piggybacks on the opencode session the user already has running — no need to start a separate `opencode serve` process. The plugin is auto-installed to `~/.config/opencode/plugin/perch-bridge.js` on first launch.

**SSE Client** mode requires running `opencode serve` separately. Use this if you prefer not to install the plugin or need to connect to a remote opencode instance.

### Module Overview

| Module | Status | Description |
|--------|--------|-------------|
| `ingest` | Implemented | Local HTTP server receiving events from the plugin |
| `sse_client` | Implemented | SSE client connecting to opencode's event stream (fallback) |
| `state_machine` | Implemented | MascotState enum with event-driven transitions |
| `notifications` | Implemented | Native OS notifications via Tauri notification plugin |
| `tray` | Implemented | System tray icon with Show/Quit menu |
| `config` | Implemented | Environment-based configuration |
| `plugin_installer` | Implemented | Auto-installs the opencode plugin on first run |
| `overlay_renderer` | Stub | Future: ASCII mascot renderer |
| `settings_store` | Stub | Future: Persistent settings via Tauri store |

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (1.70+)
- [Node.js](https://nodejs.org/) (18+)
- [Tauri CLI](https://v2.tauri.app/start/prerequisites/)

## Setup

1. Clone the repository:
   ```bash
   git clone <repository-url>
   cd perch
   ```

2. Install frontend dependencies:
   ```bash
   npm install
   ```

3. Install Tauri CLI (if not already installed):
   ```bash
   cargo install tauri-cli --version "^2"
   ```

## Development

### Running in Dev Mode

```bash
cargo tauri dev
```

This will:
- Start the Vite dev server for the frontend
- Compile and run the Rust backend
- Install the opencode plugin on first run (plugin mode)
- Start the local HTTP ingest server (plugin mode) or SSE client (SSE mode)

### Building for Production

```bash
cargo tauri build
```

## Configuration

Configure Perch via environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `PERCH_SOURCE` | `plugin` | Event source: `plugin` or `sse` |
| `PERCH_INGEST_PORT` | `4097` | Local HTTP ingest server port (plugin mode) |
| `PERCH_HOST` | `127.0.0.1` | opencode server host (SSE mode) |
| `PERCH_PORT` | `4096` | opencode server port (SSE mode) |
| `OPENCODE_PORT` | `4096` | Alternative SSE port variable (used by opencode) |

### Plugin Mode (Default)

No configuration needed — Perch starts a local HTTP server on port 4097 and installs the plugin automatically. Just run `cargo tauri dev`.

### SSE Mode (Fallback)

```bash
# Start opencode serve in one terminal
opencode serve

# Start Perch in SSE mode
PERCH_SOURCE=sse cargo tauri dev
```

## How It Works

### Plugin Bridge Mode (Default)

1. **Plugin Install**: On first launch, Perch copies `perch-bridge.js` to `~/.config/opencode/plugin/`
2. **Event Forwarding**: The plugin subscribes to opencode's `event` hook and POSTs events to `http://127.0.0.1:4097/ingest`
3. **State Tracking**: Events update the mascot state machine:
   - `session.busy` / `session.streaming` → Working
   - `session.idle` → Done (if worked >10s) or Idle (if short)
   - `session.error` → Error
4. **Debounce**: Only triggers notification after 10+ seconds of work
5. **Notification**: Fires native OS notification on transition to Done

### SSE Mode (Fallback)

Same as above, but Perch connects directly to opencode's SSE endpoint instead of receiving events via HTTP.

## Ingest Endpoint

When running in plugin mode, Perch exposes a local HTTP server:

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/ingest` | POST | Receive events from the plugin |
| `/health` | POST | Health check |

**POST /ingest** payload:
```json
{
  "event": "session.idle",
  "data": null
}
```

## Event Types

| Event | State Transition | Notes |
|-------|------------------|-------|
| `session.busy` | → Working | Starts timer |
| `session.streaming` | → Working | Starts timer |
| `session.idle` | Working → Done/Idle | Based on duration |
| `session.error` | → Error | Logs error message |

## Project Structure

```
perch/
├── src-tauri/               # Rust backend
│   ├── src/
│   │   ├── lib.rs           # Main entry point
│   │   ├── main.rs          # Binary entry point
│   │   ├── config.rs        # Configuration handling
│   │   ├── ingest.rs        # Local HTTP ingest server
│   │   ├── sse_client.rs    # SSE client (fallback)
│   │   ├── state_machine.rs # State machine
│   │   ├── notifications.rs # Notification handling
│   │   ├── plugin_installer.rs # Auto-installs opencode plugin
│   │   └── tray.rs          # System tray setup
│   ├── capabilities/        # Tauri v2 permissions
│   │   └── default.json
│   ├── Cargo.toml           # Rust dependencies
│   └── tauri.conf.json      # Tauri configuration
├── plugins/                 # OpenCode plugins
│   └── perch-bridge.js      # Event bridge plugin
├── src/                     # Frontend (minimal)
│   └── main.js
├── package.json             # Frontend dependencies
├── vite.config.js           # Vite configuration
└── index.html               # Frontend entry point
```

## Future Roadmap

- [ ] ASCII mascot renderer (overlay window)
- [ ] Settings store (persistent configuration)
- [ ] Multiple notification styles
- [ ] Windows and Linux support
- [ ] Customizable mascot states
- [ ] Sound effects

## License

MIT
