# Architecture Overview

Pulpkit POC is a WebKitGTK-based desktop shell for Wayland compositors. It renders HTML/CSS/JS themes inside layer-shell surfaces and pushes system state to them as JSON on a fixed timer.

## The 4-Surface Model

Pulpkit creates exactly four layer-shell surfaces at startup:

| Surface | Layer | Purpose |
|---------|-------|---------|
| **Bar** | `Layer::Top` | Desktop panel (top or bottom edge) |
| **Popup** | `Layer::Overlay` | Contextual panels (settings, launcher, wifi) |
| **Toast** | `Layer::Top` | Notification popups (top-right corner) |
| **Lock** | `Layer::Overlay` | Session lock screen (full screen, keyboard exclusive) |

Each surface hosts an independent `WebView` instance backed by WebKitGTK (`webkit6::WebView`). All surfaces share the same state but render independently using their own HTML documents.

!!! info "Backdrop"
    A fifth surface -- the **backdrop** -- is a transparent `Layer::Overlay` window that covers the entire screen when a popup is visible. Clicking it dismisses the popup. Its opacity is configurable via `popup_backdrop` and `popup_dim_opacity` in the theme's `config.json`.

## Data Flow

```
 Watcher Threads                Main GTK Thread                 WebViews
 ===============                ===============                 ========

 +------------------+
 | audio (pactl)    |--+
 +------------------+  |
 | network (NM)     |--+
 +------------------+  |
 | bluetooth (bluez)|--+     +-------------------+
 +------------------+  |     |                   |       +------------------+
 | upower (battery) |--+---->| Arc<Mutex<Full-   |       | bar.html         |
 +------------------+  |     | State>>           |       +------------------+
 | mpris (media)    |--+     |                   |       | popup.html       |
 +------------------+  |     | + AtomicBool      |       +------------------+
 | niri (wm events) |--+     |   (dirty flag)    |       | toast.html       |
 +------------------+  |     +--------+----------+       +------------------+
 | ... 47 more      |--+              |
 +------------------+                 |
                                      v
                              +-------+--------+
                              | 80ms GLib timer |
                              | (main thread)   |
                              +-------+--------+
                                      |
                         dirty? ------+------ not dirty?
                           |                      |
                           v                  (skip cycle)
                    +-----------+
                    | Serialize |
                    | FullState |
                    | to JSON   |
                    +-----------+
                           |
                           v
                   JSON === last_json?
                     |              |
                  (same)        (different)
                     |              |
                  (skip)            v
                           +----------------+
                           | Evaluate JS in |
                           | all WebViews:  |
                           | updateState({})|
                           +----------------+
```

## Render Loop

The render loop runs on the **main GTK thread** as a `glib::timeout_add_local` callback every **80ms** (12.5 Hz):

1. **Check dirty flag** -- `AtomicBool::swap(false)`. If not dirty, return immediately.
2. **Merge state** -- Lock `FullState` from polled_state, overlay AppState fields (popup, theme, wifi_nets, apps, tray_items, custom).
3. **Serialize to JSON** -- `serde_json::to_string(&state)`.
4. **Deduplicate** -- Compare JSON string against previous frame. Skip if identical.
5. **Push to WebViews** -- Evaluate `updateState({...})` in bar, toast, and (if visible) popup WebViews.
6. **Toggle popup visibility** -- Show/hide the popup and backdrop windows based on `state.popup`.

!!! note "Why 80ms?"
    80ms gives smooth visual updates (12.5 fps) while keeping CPU usage near zero when idle. The timer only serializes and pushes when the dirty flag is set -- most cycles are no-ops.

## Thread Model

```
Main GTK Thread
  |-- GLib event loop
  |-- 80ms state push timer
  |-- 50ms IPC message processor
  |-- WebView rendering (all 4 surfaces)
  |-- Command handler (JS -> Rust via UserContentManager)

Watcher Threads (1 per watcher, ~53 total)
  |-- Each holds Arc<Mutex<FullState>> + Arc<AtomicBool>
  |-- Block on DBus signals, stream stdout, or sleep loops
  |-- Write to FullState, set dirty flag

Tokio Threads (spawned inside std::thread for async DBus services)
  |-- Notification daemon (org.freedesktop.Notifications)
  |-- Shell DBus service (org.pulpkit.Shell)
  |-- System tray client (StatusNotifierWatcher)

Background Poller Thread
  |-- Polls CPU, memory, disk, uptime every 3 seconds
  |-- These are the only remaining polled values
```

!!! warning "No async on the main thread"
    The main GTK thread is synchronous. All async work (tokio runtimes) runs in dedicated `std::thread::spawn` threads. The `zbus::blocking` API is used for all DBus watchers that don't need to serve interfaces.

## IPC

Pulpkit listens on a Unix domain socket for external tools (CLI, devkit, editor integrations):

- **Path**: `$XDG_RUNTIME_DIR/pulpkit.sock` (fallback: `/tmp/pulpkit.sock`)
- **Protocol**: Newline-delimited JSON request/response
- **Thread**: Dedicated listener thread, one handler thread per connection

### IPC Methods

| Method | Description |
|--------|-------------|
| `get_state` | Returns the full serialized state as JSON |
| `reload_bar` | Hot-reload bar HTML from path or inline HTML |
| `reload_popup` | Hot-reload popup HTML from path or inline HTML |
| `eval_js` | Evaluate arbitrary JS in a target WebView |
| `set_mock_state` | Push mock state to all WebViews (devkit) |
| `get_console_logs` | Returns WebView console log buffer |

### DBus Service

Pulpkit also exposes an `org.pulpkit.Shell` service on the session bus:

| Method | Signature | Description |
|--------|-----------|-------------|
| `GetState` | `() -> s` | Full state as JSON string |
| `SetCustom` | `(ss)` | Set a key-value pair in the custom store |
| `TogglePopup` | `(s)` | Toggle a named popup |
| `Dismiss` | `()` | Close the current popup |
| `Exec` | `(s)` | Run a shell command |

## Command Flow (JS to Rust)

Themes send commands to the shell via the WebKit message handler:

```javascript
// In theme JS
window.webkit.messageHandlers.pulpkit.postMessage(
  JSON.stringify({ cmd: "vol_set", data: 75 })
);
```

The message handler on the Rust side (`UserContentManager::connect_script_message_received`) routes to `handle_command()`, which dispatches ~30 commands including volume control, brightness, workspace switching, power management, and popup toggling.

## Theme Loading

Themes are loaded at startup from `shells/<name>/`:

```
shells/
  mocha/
    bar.html          # Bar surface markup
    popup.html        # Popup surface markup
    toast.html        # Toast surface markup (optional)
    config.json       # Theme dimensions and behavior
```

The `config.json` controls:

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `bar_height` | `i32` | `40` | Bar surface height in pixels |
| `popup_width` | `i32` | `380` | Popup surface width |
| `popup_height` | `i32` | `500` | Popup surface height |
| `bar_position` | `String` | `"top"` | `"top"` or `"bottom"` |
| `popup_backdrop` | `String` | `"dim"` | `"dim"`, `"none"`, or `"blur"` |
| `popup_dim_opacity` | `f32` | `0.25` | Backdrop dim opacity |
