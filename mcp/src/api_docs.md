# Pulpkit Shell API

## State Fields (pushed to JS as `updateState(s)`)

State is pushed ~80ms after any reactive change.

### Audio

| Field | Type | Description |
|---|---|---|
| `s.vol` | 0-100 | Audio volume |
| `s.muted` | bool | Audio muted |
| `s.audio_device` | string | Output device name |
| `s.audio_sinks` | array | `[{name, description, active, volume, muted}]` — output devices |
| `s.audio_sources` | array | `[{name, description, active, volume, muted}]` — input devices |
| `s.audio_streams` | array | `[{name, app_name, volume, muted, is_input}]` — Pipewire streams |

### Display

| Field | Type | Description |
|---|---|---|
| `s.bright` | 0-100 | Screen brightness |
| `s.night_light_active` | bool | Night light (wlsunset) enabled |
| `s.screen_sharing` | bool | Screen sharing active |
| `s.outputs` | array | `[{name, make, model, width, height, refresh, scale, enabled}]` — display outputs |

### Battery & Power

| Field | Type | Description |
|---|---|---|
| `s.bat` | 0-100 | Battery percentage |
| `s.bat_status` | string | "Charging" / "Discharging" / "Full" / "Not charging" |
| `s.has_bat` | bool | Whether device has a battery |
| `s.ac_plugged` | bool | AC power supply connected |
| `s.power_profile` | string | "balanced" / "performance" / "power-saver" |

### System

| Field | Type | Description |
|---|---|---|
| `s.cpu` | 0-100 | CPU usage |
| `s.mem` | 0-100 | RAM usage |
| `s.disk_used` | string | e.g. "171G" |
| `s.disk_total` | string | e.g. "248G" |
| `s.disk_pct` | 0-100 | Disk usage percentage |
| `s.cpu_temp` | u32 | CPU temperature (degrees C) |
| `s.swap_used_mb` | u32 | Swap used in MB |
| `s.swap_total_mb` | u32 | Swap total in MB |
| `s.load_1` | f32 | 1-minute load average |
| `s.load_5` | f32 | 5-minute load average |
| `s.load_15` | f32 | 15-minute load average |
| `s.top_procs` | array | `[{name, pid, cpu_pct, mem_mb}]` — top processes |
| `s.uptime` | string | System uptime |
| `s.timezone` | string | System timezone |
| `s.compositor` | string | Compositor name (e.g. "niri", "sway", "hyprland") |

### Network

| Field | Type | Description |
|---|---|---|
| `s.wifi` | string | Connected SSID or "" |
| `s.net_signal` | 0-100 | WiFi signal strength |
| `s.net_ip` | string | IP address |
| `s.net_rx_bytes_sec` | u64 | Network download bytes/sec |
| `s.net_tx_bytes_sec` | u64 | Network upload bytes/sec |
| `s.wifi_nets` | array | `[{ssid, signal, secure, active}]` — available networks |

### Bluetooth

| Field | Type | Description |
|---|---|---|
| `s.bt_powered` | bool | Bluetooth adapter powered on |
| `s.bt_connected` | array | `[{name, address, connected, icon}]` — paired/connected devices |

### Notifications

| Field | Type | Description |
|---|---|---|
| `s.notif_count` | u32 | Number of pending notifications |
| `s.dnd` | bool | Do Not Disturb mode active |
| `s.notifications` | array | `[{id, app_name, summary, body, icon, timestamp}]` — notification list |

### MPRIS Media

| Field | Type | Description |
|---|---|---|
| `s.media_playing` | bool | Media currently playing |
| `s.media_title` | string | Track title |
| `s.media_artist` | string | Track artist |
| `s.media_album` | string | Track album |
| `s.media_art_url` | string | Album art URL |
| `s.media_player` | string | Player name (e.g. "spotify") |

### Workspaces & Windows

| Field | Type | Description |
|---|---|---|
| `s.ws` | array | `[{idx: number, active: bool}]` — workspaces |
| `s.windows` | array | `[{id, title, app_id, focused, icon}]` — running windows |
| `s.active_title` | string | Focused window title |
| `s.active_app_id` | string | Focused window app_id |

### Session (logind)

| Field | Type | Description |
|---|---|---|
| `s.session_locked` | bool | Session is locked |
| `s.session_idle` | bool | Session is idle |
| `s.preparing_sleep` | bool | System preparing to sleep |

### Gaming

| Field | Type | Description |
|---|---|---|
| `s.gamemode_active` | bool | GameMode enabled |
| `s.gpu_usage` | 0-100 | GPU usage |
| `s.gpu_temp` | u32 | GPU temperature (degrees C) |
| `s.vram_used_mb` | u32 | VRAM used in MB |
| `s.vram_total_mb` | u32 | VRAM total in MB |
| `s.gamescope_active` | bool | Gamescope session active |
| `s.discord_activity` | string | Current Discord Rich Presence activity |

### Keyboard & Input

| Field | Type | Description |
|---|---|---|
| `s.kb_layout` | string | Keyboard layout (e.g. "us") |
| `s.kb_variant` | string | Keyboard variant |
| `s.im_active` | bool | Input method (Fcitx/IBus) active |
| `s.im_name` | string | Input method name |

### Clipboard

| Field | Type | Description |
|---|---|---|
| `s.clipboard_text` | string | Current clipboard text content |

### Removable Drives

| Field | Type | Description |
|---|---|---|
| `s.drives` | array | `[{name, mount_point, size_bytes, device}]` — removable drives |

### Idle Inhibitors

| Field | Type | Description |
|---|---|---|
| `s.inhibitors` | array | `[{who, why, what}]` — active idle inhibitors |

### Systemd

| Field | Type | Description |
|---|---|---|
| `s.failed_units` | array | List of failed systemd unit names |
| `s.failed_unit_count` | u32 | Number of failed systemd units |

### Package Updates

| Field | Type | Description |
|---|---|---|
| `s.updates_available` | u32 | Number of available package updates |

### Trash

| Field | Type | Description |
|---|---|---|
| `s.trash_count` | u32 | Number of items in trash |

### Polkit

| Field | Type | Description |
|---|---|---|
| `s.polkit_pending` | bool | Polkit authorization pending |
| `s.polkit_message` | string | Polkit authorization message |

### Weather

| Field | Type | Description |
|---|---|---|
| `s.weather_temp` | f32 | Temperature |
| `s.weather_condition` | string | Condition description |
| `s.weather_icon` | string | Weather icon name |

### Calendar

| Field | Type | Description |
|---|---|---|
| `s.calendar_events` | array | `[{summary, start, end, location}]` — iCal events |

### Recent Files

| Field | Type | Description |
|---|---|---|
| `s.recent_files` | array | `[{name, uri, mime_type, timestamp}]` — recently opened files |

### Applications & Tray

| Field | Type | Description |
|---|---|---|
| `s.apps` | array | `[{name, exec, icon}]` — installed applications |
| `s.tray_items` | array | `[{id, address, title, icon}]` — system tray |

### UI State

| Field | Type | Description |
|---|---|---|
| `s.popup` | string | Current open popup name or "" |
| `s.theme` | string | Current color theme name |
| `s.custom` | object | Arbitrary key-value store |

### User & Host

| Field | Type | Description |
|---|---|---|
| `s.user` | string | Username |
| `s.user_icon` | string | User avatar path |
| `s.host` | string | Hostname |
| `s.kernel` | string | Kernel version |

## Commands (JS → Rust via `send({cmd, data})`)

```js
// Send command helper (defined in shell HTML):
function send(o) {
  window.webkit.messageHandlers.pulpkit.postMessage(JSON.stringify(o));
}
```

| Command | Data | Description |
|---|---|---|
| `vol_set` | number 0-100 | Set volume |
| `vol_mute` | — | Toggle mute |
| `bri_set` | number 0-100 | Set brightness |
| `set_profile` | "balanced"/"performance"/"power-saver" | Set power profile |
| `wifi_con` | "SSID" | Connect to WiFi |
| `wifi_dis` | — | Disconnect WiFi |
| `ws_go` | number | Switch workspace |
| `focus_window` | number (id) | Focus a window |
| `close_window` | number (id) | Close a window |
| `move_to_workspace` | {id, ws} | Move window to workspace |
| `launch` | "command" | Launch application |
| `exec` | "shell command" | Run arbitrary command |
| `screenshot` | — | Region screenshot (grim+slurp) |
| `screenshot_full` | — | Full screen screenshot |
| `toggle_night` | — | Toggle night light (wlsunset) |
| `toggle_bt` | — | Toggle bluetooth |
| `toggle_dnd` | — | Toggle Do Not Disturb (mako) |
| `notif_dismiss` | — | Dismiss all notifications |
| `notif_dismiss_one` | — | Dismiss latest notification |
| `power_lock` | — | Lock screen |
| `power_suspend` | — | Suspend |
| `power_reboot` | — | Reboot |
| `power_shutdown` | — | Shut down |
| `power_logout` | — | Log out (niri quit) |
| `popup` | "settings"/"wifi"/"power"/"launcher"/"config" | Toggle popup |
| `dismiss` | — | Close any open popup |
| `set_theme` | theme name | Switch color theme |
| `set_custom` | {key, value} | Store custom state |
| `tray_activate` | {address, click: "left"/"right"} | Activate tray item |

## Color Themes

Available: mocha, macchiato, frappe, latte, tokyonight, nord, gruvbox, rosepine, onedark, dracula, solarized, flexoki

CSS variables (set via `data-theme` attribute on `<html>`):
`--bg`, `--bg-surface`, `--bg-overlay`, `--fg`, `--fg-muted`, `--fg-dim`, `--accent`, `--blue`, `--green`, `--red`, `--yellow`, `--peach`, `--teal`, `--pink`, `--mauve`, `--text-on-color`

## HTML Contract

**bar.html:**
- Must define `function updateState(s)` — called on every state push
- Must define `function send(o)` — sends commands to Rust
- Receives full state object ~80ms after any reactive change

**popup.html:**
- Same contract as bar.html
- Panels use `id="panel-{name}"` with CSS class `active` to show/hide
- Supported panels: settings, wifi, power, launcher, config (or any custom name)

**Theme application:**
```js
// In updateState:
if (s.theme) document.documentElement.setAttribute('data-theme', s.theme);
```

## DBus Services

### `org.pulpkit.Shell` (session bus)

Exposed by the shell daemon. External tools can control the shell over DBus.

| Method | Arguments | Returns | Description |
|---|---|---|---|
| `GetState` | — | JSON string | Returns the full serialized state |
| `SetCustom` | key: string, value: string | — | Set a custom state key-value pair |
| `TogglePopup` | name: string | — | Toggle a popup panel by name |
| `Dismiss` | — | — | Close any open popup |
| `Exec` | command: string | — | Run a shell command |

### `org.freedesktop.Notifications` (session bus)

Pulpkit implements the XDG notification daemon spec. Notifications received are stored in `s.notifications` and counted in `s.notif_count`.

| Method | Arguments | Description |
|---|---|---|
| `Notify` | (standard XDG args) | Receive and display a notification |
| `CloseNotification` | id: u32 | Close a notification by ID |
| `GetCapabilities` | — | Returns supported capabilities |
| `GetServerInformation` | — | Returns daemon name/version |

## Shell Theme Config (config.json)

```json
{
  "bar_height": 40,
  "bar_position": "top",  // or "bottom"
  "popup_width": 380,
  "popup_height": 500
}
```
