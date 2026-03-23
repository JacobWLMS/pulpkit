# Quick Start

## Clone, Build, Run

```bash
git clone https://github.com/JacobWLMS/pulpkit.git
cd pulpkit
cargo build --release -p pulpkit-webshell-poc
./target/release/pulpkit-webshell-poc zenith
```

This launches the **Zenith** shell — the reference implementation. Other built-in shells: `glass`, `minimal`, `neon`, `gruvbox-rice`.

```bash
# Try a different shell
./target/release/pulpkit-webshell-poc minimal
```

## How It Works

Pulpkit creates up to four layer-shell surfaces, each a WebKitGTK webview:

| Surface | File | Purpose |
|---------|------|---------|
| **Bar** | `bar.html` | Status bar — always visible |
| **Popup** | `popup.html` | Panels (settings, wifi, launcher, power) |
| **Toast** | `toast.html` | Notification toasts |
| **Lock** | `lock.html` | PAM-authenticated lock screen |

Each surface loads its HTML file and receives the same state object.

## State Flow

The Rust backend runs **53 watchers** that monitor system state — audio levels, network, bluetooth, workspaces, GPU, media, and more. Every **80ms**, the backend:

1. Serializes all 187 state fields into a single JSON object
2. Calls `updateState(s)` on every active surface
3. Your JavaScript reads the fields it cares about and updates the DOM

```
Watchers → FullState struct → JSON → updateState(s) → your DOM
```

!!! info "Why 80ms?"
    80ms gives ~12.5 updates per second — fast enough that volume sliders and workspace switches feel instant, slow enough to keep CPU usage negligible. Watchers themselves are event-driven (DBus signals, inotify, socket reads), so changes are captured immediately. The 80ms interval is only the serialization and delivery cadence.

## Opening Popups

Your HTML sends commands back to the Rust backend through a WebKit message handler:

```javascript
function send(o) {
  window.webkit.messageHandlers.pulpkit.postMessage(JSON.stringify(o));
}

// Toggle a popup panel
send({ cmd: 'popup', data: 'settings' });

// Close the current popup
send({ cmd: 'dismiss' });
```

When a popup opens, Pulpkit shows the popup surface and (optionally) a backdrop surface that dims the screen. Clicking the backdrop dismisses the popup.

The `s.popup` field in `updateState` tells you which panel is currently active, so your `popup.html` can show/hide the right content:

```javascript
function updateState(s) {
  // Hide all panels
  document.querySelectorAll('.panel').forEach(p => p.classList.remove('active'));

  // Show the active one
  const panel = document.getElementById('panel-' + s.popup);
  if (panel) panel.classList.add('active');
}
```

## Sending Commands

Commands cover everything from volume control to launching apps:

```javascript
send({ cmd: 'vol_set', data: 75 });        // Set volume to 75%
send({ cmd: 'vol_mute' });                  // Toggle mute
send({ cmd: 'bri_set', data: 50 });         // Set brightness to 50%
send({ cmd: 'ws_go', data: 3 });            // Switch to workspace 3
send({ cmd: 'launch', data: 'firefox' });   // Launch an application
send({ cmd: 'set_theme', data: 'nord' });   // Switch color theme
send({ cmd: 'screenshot' });                // Take a region screenshot
send({ cmd: 'power_lock' });                // Lock the session
send({ cmd: 'power_suspend' });             // Suspend the system
```

## Next Steps

Ready to build your own shell? Head to [Your First Shell](first-shell.md).
