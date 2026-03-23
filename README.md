<div align="center">

# Pulpkit

**A Wayland desktop shell framework where your UI is just HTML/CSS/JS.**

[![Rust](https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)]()
[![Wayland](https://img.shields.io/badge/Wayland-FFB800?style=for-the-badge&logo=wayland&logoColor=black)]()
[![GTK4](https://img.shields.io/badge/GTK4-4A86CF?style=for-the-badge&logo=gtk&logoColor=white)]()
[![License: MIT](https://img.shields.io/badge/License-MIT-blue?style=for-the-badge)](LICENSE)

<!-- screenshot placeholder -->

</div>

---

## What is Pulpkit?

Pulpkit is a desktop shell framework for Wayland compositors. It uses **GTK4 layer-shell** for window management and **WebKitGTK** for rendering, which means your bar, popups, notification toasts, and lock screen are plain HTML/CSS/JS files. The Rust backend runs **53 reactive system watchers** that push **187 state fields** to your JavaScript as a single JSON object. Built-in notification daemon, system tray, PAM-based lock screen, DBus service, IPC socket, and an MCP server for AI-assisted shell development. You write the frontend. Pulpkit handles everything else.

---

## Features

<table>
<tr>
<td width="50%" valign="top">

### 53 Reactive Watchers

System state is pushed to your JS automatically. No polling required.

| Category | Watchers |
|----------|----------|
| **Audio** | Volume, mute, sinks, sources, streams, mic |
| **Network** | WiFi, IP, signal, speed (rx/tx), VPN |
| **Bluetooth** | Power, connected devices |
| **Media** | MPRIS (title, artist, album, art, player) |
| **Battery** | Level, status, AC power, power draw |
| **GPU** | Usage, temp, VRAM |
| **Gaming** | GameMode, Gamescope, Discord activity |
| **Display** | Brightness, outputs, night light, screen share |
| **System** | CPU, memory, swap, disk, load avg, thermal, fan |
| **Session** | Lock, idle, sleep, inhibitors, caffeine |
| **Systemd** | Failed units, timers, journal errors |
| **Desktop** | Workspaces, windows, tray, keyboard layout |
| **Other** | Weather, calendar, containers, packages, trash, SSH, clipboard |

</td>
<td width="50%" valign="top">

### Core Capabilities

**Built-in Notification Daemon** --- Replaces mako, dunst, swaync. Notifications appear in a dedicated toast surface. Full control from JS.

**DBus Service** --- Exposes `org.pulpkit.Shell` for external tools and scripts to interact with the shell.

**IPC Socket** --- Unix socket at `$XDG_RUNTIME_DIR/pulpkit.sock` for command-line control and the MCP server.

**4 Layer-Shell Surfaces** --- Bar, popup, toast, and backdrop. Each is a WebKitGTK webview with transparent backgrounds.

**Lock Screen** --- PAM authentication with greetd support. Your `lock.html` becomes the lock screen.

**Configurable Backdrop** --- `none`, `dim`, or `opaque` with adjustable opacity when popups are open.

**12 Color Themes** --- CSS variable-based theming. Switch at runtime with a single command.

**MCP Server** --- AI agents can read state, hot-reload HTML, eval JS, take screenshots, and scaffold new shells.

</td>
</tr>
</table>

---

## Architecture

```
                         ┌──────────────────────────────────────────┐
                         │              Rust Backend                │
                         │                                         │
                         │  ┌─────────────────────────────────┐    │
                         │  │       53 System Watchers         │    │
                         │  │  (DBus, /sys, sockets, polling)  │    │
                         │  └──────────────┬──────────────────┘    │
                         │                 │                        │
                         │                 ▼                        │
                         │  ┌─────────────────────────────────┐    │
                         │  │     FullState (187 fields)       │    │
                         │  │        serialize → JSON          │    │
                         │  └──────────────┬──────────────────┘    │
                         │                 │                        │
                         └─────────────────┼────────────────────────┘
                                           │
                           updateState(s)  │  send({cmd, data})
                                           ▼
               ┌───────────────────────────────────────────────────┐
               │               WebKitGTK Surfaces                  │
               │                                                   │
               │   ┌─────────┐  ┌─────────┐  ┌───────┐  ┌──────┐ │
               │   │   Bar   │  │  Popup  │  │ Toast │  │ Lock │ │
               │   │  .html  │  │  .html  │  │ .html │  │ .html│ │
               │   └─────────┘  └─────────┘  └───────┘  └──────┘ │
               │           (GTK4 Layer-Shell surfaces)             │
               └───────────────────────────────────────────────────┘
```

Each surface is a full WebKitGTK webview. The Rust backend serializes the entire `FullState` struct to JSON and calls `updateState(s)` on every surface. Your JS sends commands back via `send({cmd, data})`, which posts a message through WebKit's native bridge.

---

## Quick Start

```bash
# Dependencies (Arch / CachyOS)
sudo pacman -S gtk4 gtk4-layer-shell webkit2gtk-6.0

# Build
cargo build --release -p pulpkit-webshell-poc

# Run with the Zenith shell
./target/release/pulpkit-webshell-poc zenith
```

Other included shells: `glass`, `minimal`, `neon`, `gruvbox-rice`.

---

## Creating Your Own Shell

Create a directory under `poc/shells/yourname/` with these files:

```
poc/shells/yourname/
├── bar.html          # Status bar (required)
├── popup.html        # Popup panels (required)
├── toast.html        # Notification toasts (optional)
├── lock.html         # Lock screen (optional)
├── config.json       # Dimensions and positioning
└── theme.css         # Custom styles (optional)
```

### The Contract

Every HTML file must define two functions:

```js
// Called by Pulpkit whenever state changes
function updateState(s) {
  // s.vol, s.bat, s.wifi, s.media_title, s.notifications, ...
  // Apply theme:
  document.documentElement.setAttribute('data-theme', s.theme);
}

// Send commands back to the Rust backend
function send(o) {
  window.webkit.messageHandlers.pulpkit.postMessage(JSON.stringify(o));
}
```

### Commands

```js
send({ cmd: 'vol_set', data: 75 });          // Set volume
send({ cmd: 'popup', data: 'settings' });     // Toggle popup panel
send({ cmd: 'dismiss' });                     // Close popup
send({ cmd: 'set_theme', data: 'nord' });     // Switch theme
send({ cmd: 'launch', data: 'firefox' });     // Launch app
send({ cmd: 'screenshot' });                   // Region screenshot
send({ cmd: 'power_lock' });                   // Lock session
```

### config.json

```json
{
  "bar_height": 40,
  "bar_position": "top",
  "popup_width": 380,
  "popup_height": 500,
  "popup_backdrop": "dim",
  "popup_dim_opacity": 0.25
}
```

---

## State Fields

The full state object has **187 fields** across 17 categories. Here are some highlights:

| Field | Type | Description |
|-------|------|-------------|
| `s.media_title` | string | Currently playing track |
| `s.media_artist` | string | Artist name (MPRIS) |
| `s.media_art_url` | string | Album art URL |
| `s.gpu_usage` | 0-100 | GPU utilization |
| `s.gpu_temp` | number | GPU temperature (C) |
| `s.vram_used_mb` | number | VRAM in use |
| `s.gamescope_active` | bool | Gamescope session detected |
| `s.gamemode_active` | bool | Feral GameMode status |
| `s.discord_activity` | string | Current Discord activity |
| `s.containers` | array | Docker/Podman containers |
| `s.notifications` | array | Active notifications |
| `s.audio_streams` | array | Per-app audio streams (Pipewire) |
| `s.top_procs` | array | Top processes by CPU |
| `s.calendar_events` | array | Upcoming calendar events |
| `s.weather_temp` | number | Current temperature |
| `s.vpn_active` | bool | VPN connection state |
| `s.failed_units` | array | Failed systemd units |
| `s.power_draw_watts` | number | System power consumption |
| `s.net_rx_bytes_sec` | number | Network download speed |
| `s.caffeine_active` | bool | Manual idle inhibit |

Full API documentation is available via `send({cmd: 'get_api_docs'})` or the MCP server's `get_api_docs` tool.

---

## Themes

Pulpkit ships 12 color themes, switchable at runtime. Each theme sets 16 CSS variables (`--bg`, `--fg`, `--accent`, `--blue`, `--green`, `--red`, etc.).

| Theme | Origin |
|-------|--------|
| `mocha` | Catppuccin Mocha |
| `macchiato` | Catppuccin Macchiato |
| `frappe` | Catppuccin Frappe |
| `latte` | Catppuccin Latte |
| `tokyonight` | Tokyo Night |
| `nord` | Nord |
| `gruvbox` | Gruvbox |
| `rosepine` | Rose Pine |
| `onedark` | One Dark |
| `dracula` | Dracula |
| `solarized` | Solarized |
| `flexoki` | Flexoki |

```js
// Switch theme at runtime
send({ cmd: 'set_theme', data: 'rosepine' });
```

```css
/* Use theme variables in your CSS */
.bar { background: var(--bg); color: var(--fg); }
.accent { color: var(--accent); }
.warning { color: var(--yellow); }
```

---

## Zenith

<!-- screenshot placeholder -->

See [Zenith](https://github.com/JacobWLMS/zenith) for the reference shell implementation.

---

## MCP Server

Pulpkit includes an MCP (Model Context Protocol) server that lets AI agents design and iterate on shells in real time.

```bash
# Build the MCP server
cargo build --release -p pulpkit-mcp
```

The server communicates with the running shell over the IPC socket and exposes tools for:

- **`get_state`** --- Read all 187 state fields from the live shell
- **`hot_reload_bar`** / **`hot_reload_popup`** --- Push new HTML to surfaces without restarting
- **`eval_js`** --- Execute JavaScript in any webview
- **`screenshot`** --- Capture the current screen
- **`scaffold_shell`** --- Generate a new shell project with component structure
- **`preview_shell`** --- Preview a shell in a browser with mock data
- **`validate_shell`** --- Check for common mistakes before deploying

Point your AI coding tool at the MCP server and say "make me a bar." It can read state, write HTML, hot-reload, screenshot the result, and iterate --- all without you touching a file.

---

## License

[MIT](LICENSE)
