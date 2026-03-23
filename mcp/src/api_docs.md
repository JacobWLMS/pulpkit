# Pulpkit Shell API

## State Fields (pushed to JS as `updateState(s)`)

| Field | Type | Description |
|---|---|---|
| `s.vol` | 0-100 | Audio volume |
| `s.muted` | bool | Audio muted |
| `s.audio_device` | string | Output device name |
| `s.bright` | 0-100 | Screen brightness |
| `s.bat` | 0-100 | Battery percentage |
| `s.bat_status` | string | "Charging" / "Discharging" / "Full" / "Not charging" |
| `s.has_bat` | bool | Whether device has a battery |
| `s.cpu` | 0-100 | CPU usage |
| `s.mem` | 0-100 | RAM usage |
| `s.disk_used` | string | e.g. "171G" |
| `s.disk_total` | string | e.g. "248G" |
| `s.disk_pct` | 0-100 | Disk usage percentage |
| `s.power_profile` | string | "balanced" / "performance" / "power-saver" |
| `s.notif_count` | u32 | Number of pending notifications |
| `s.dnd` | bool | Do Not Disturb mode active |
| `s.wifi` | string | Connected SSID or "" |
| `s.net_signal` | 0-100 | WiFi signal strength |
| `s.net_ip` | string | IP address |
| `s.ws` | array | `[{idx: number, active: bool}]` — workspaces |
| `s.windows` | array | `[{id, title, app_id, focused, icon}]` — running windows |
| `s.active_title` | string | Focused window title |
| `s.active_app_id` | string | Focused window app_id |
| `s.wifi_nets` | array | `[{ssid, signal, secure, active}]` — available networks |
| `s.apps` | array | `[{name, exec, icon}]` — installed applications |
| `s.tray_items` | array | `[{id, address, title, icon}]` — system tray |
| `s.popup` | string | Current open popup name or "" |
| `s.theme` | string | Current color theme name |
| `s.custom` | object | Arbitrary key-value store |
| `s.user` | string | Username |
| `s.host` | string | Hostname |
| `s.kernel` | string | Kernel version |
| `s.uptime` | string | System uptime |

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
- Receives full state object ~every 1.5s + immediately on UI interactions

**popup.html:**
- Same contract as bar.html
- Panels use `id="panel-{name}"` with CSS class `active` to show/hide
- Supported panels: settings, wifi, power, launcher, config (or any custom name)

**Theme application:**
```js
// In updateState:
if (s.theme) document.documentElement.setAttribute('data-theme', s.theme);
```

## Shell Theme Config (config.json)

```json
{
  "bar_height": 40,
  "bar_position": "top",  // or "bottom"
  "popup_width": 380,
  "popup_height": 500
}
```
