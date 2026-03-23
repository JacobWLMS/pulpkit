# Panels

Zenith has 10 popup panels, each rendered inside the shared `popup.html` surface. When a panel is opened, Pulpkit shows the popup surface and a dim backdrop (`rgba(0,0,0,0.25)`). Clicking the backdrop or pressing Escape dismisses the popup.

Every panel is opened by sending a popup command:

```javascript
send({ cmd: 'popup', data: '<panel-name>' });
```

The `s.popup` state field tells `popup.html` which panel is currently active. Each panel div has an `id` of `panel-<name>` and is shown/hidden via the `.active` class.

---

## 1. Quick Settings

**Panel ID:** `settings`
**Opened by:** Clicking the volume icon or settings gear in the bar

The main control surface. Provides fast access to toggles, sliders, and battery status.

### Tiles (4x3 grid)

| Tile | Action | Active when |
|------|--------|-------------|
| WiFi | Opens WiFi panel | `s.wifi` is truthy |
| Bluetooth | Toggles bluetooth | `s.bt_powered` is true |
| DND | Toggles Do Not Disturb | `s.dnd` is true |
| Night Light | Toggles night light | Night light active |
| Mute | Toggles audio mute | `s.muted` is true |
| Screenshot | Takes region screenshot | — |
| Power Profile | Cycles power-saver/balanced/performance | Shows current `s.power_profile` |
| Config | Opens Settings App | — |
| Caffeine | Toggles idle inhibit | `s.caffeine_active` is true |
| VPN | Shows VPN status | `s.vpn_name` is truthy |
| Mixer | Opens Audio Mixer panel | — |

### Sliders

| Slider | State field | Command |
|--------|-------------|---------|
| Volume | `s.vol`, `s.muted` | `vol_set` |
| Brightness | `s.bright` | `bri_set` |
| Mic Volume | `s.mic_volume` | exec `wpctl set-volume` |

### Battery Row

Displays battery icon, percentage (`s.bat`), charge status (`s.bat_status`), and power draw.

---

## 2. WiFi

**Panel ID:** `wifi`
**Opened by:** Clicking the network icon in the bar, or the WiFi tile in Quick Settings

Displays available WiFi networks and the current connection.

### What it shows

- Panel title with WiFi icon
- List of scanned networks from `s.wifi_networks[]`
- Each network shows: signal strength icon, SSID, signal percentage, security type
- Connected network highlighted in green with bold SSID
- Click a network to connect; click the connected network to disconnect

### State fields read

`s.wifi`, `s.wifi_signal`, `s.wifi_networks`

---

## 3. Power

**Panel ID:** `power`
**Opened by:** Clicking the power icon in the bar

Session controls and system power actions.

### What it shows

- User info header: hostname (`s.hostname`) and username (`s.username`) with uptime
- Menu items with Nerd Font icons:

| Action | Icon | Command |
|--------|------|---------|
| Lock | Lock icon | `power_lock` |
| Suspend | Suspend icon | `power_suspend` |
| Log Out | Logout icon | `power_logout` |
| Reboot | Reboot icon | `power_reboot` |
| Shut Down | Shutdown icon (red) | `power_shutdown` |

### State fields read

`s.hostname`, `s.username`, `s.uptime`

---

## 4. Launcher

**Panel ID:** `launcher`
**Opened by:** Clicking the launcher button (top-left icon) in the bar

Application launcher with search and keyboard navigation.

### What it shows

- Search input field (auto-focused on open)
- Scrollable list of applications from `s.apps[]`
- Each app shows: icon (or fallback initial), app name
- Selected app highlighted with accent-colored left border

### Interaction

- Type to filter apps by name
- Arrow keys to navigate the list
- Enter to launch the selected app (`launch` command with app ID)
- Escape to dismiss

### State fields read

`s.apps`

---

## 5. Audio Mixer

**Panel ID:** `audio`
**Opened by:** Clicking the Mixer tile in Quick Settings

Full audio control with device selection and per-app volume.

### What it shows

- **Output device selector** — Lists all audio sinks from `s.audio_sinks[]`, active device highlighted. Click to switch output device.
- **Input device selector** — Lists all audio sources from `s.audio_sources[]`. Click to switch input device.
- **Master volume slider** — Controls system volume (`vol_set` command)
- **Mic volume slider** — Controls microphone level
- **Per-app streams** — Each entry from `s.audio_streams[]` shows: app name, volume slider, mute toggle. Adjust individual app volumes independently.

### State fields read

`s.vol`, `s.muted`, `s.mic_volume`, `s.audio_device`, `s.audio_sinks`, `s.audio_sources`, `s.audio_streams`

---

## 6. Calendar / Weather

**Panel ID:** `calendar`
**Opened by:** Clicking the clock in the bar center

Month calendar view with weather information.

### What it shows

- **Header** — Current month and year with left/right navigation arrows
- **Calendar grid** — 7-column grid (Mon-Sun), today highlighted with accent-colored circle
- **Today section** — Full date string for today
- **Weather row** — Temperature, condition, weather icon, sunrise/sunset times from `s.weather_temp`, `s.weather_condition`, `s.sunrise`, `s.sunset`
- **Events** — Upcoming calendar events from `s.calendar_events[]` (if any)
- **Timezone** — Current timezone display

### State fields read

`s.weather_temp`, `s.weather_condition`, `s.sunrise`, `s.sunset`, `s.calendar_events`, `s.timezone`

---

## 7. Notifications

**Panel ID:** `notifications`
**Opened by:** Clicking the bell icon in the bar

Notification center showing all received notifications.

### What it shows

- **Header** — "Notifications" title with notification count badge and "Clear All" button
- **DND status** — Do Not Disturb indicator with toggle
- **Notification list** — Each notification from `s.notifications[]` shows:
    - App icon (or fallback initial)
    - App name
    - Summary (bold)
    - Body text (truncated)
    - Relative timestamp ("2m ago", "1h ago")
    - Dismiss button (X) per notification
- **Empty state** — "No notifications" message when the list is empty

### Actions

| Button | Command |
|--------|---------|
| Clear All | `notif_dismiss` (dismisses all) |
| Dismiss one | `notif_close` with notification ID |
| Toggle DND | `toggle_dnd` |

### State fields read

`s.notifications`, `s.notif_count`, `s.dnd`

---

## 8. System Monitor

**Panel ID:** `monitor`
**Opened by:** Clicking the CPU or memory mini-stat in the bar

Full system monitoring dashboard for power users.

### Gauges

Each gauge shows a colored progress bar (green < 50%, accent 50-80%, yellow 80-90%, red > 90%) with a percentage value.

| Gauge | State field |
|-------|-------------|
| CPU | `s.cpu` |
| Memory | `s.mem` |
| Disk | `s.disk` |
| GPU | `s.gpu_usage` |

### Additional metrics

| Metric | State field | Icon |
|--------|-------------|------|
| CPU Temperature | `s.cpu_temp` | Thermometer |
| GPU Temperature | `s.gpu_temp` | Thermometer |
| GPU VRAM | `s.gpu_vram_used`, `s.gpu_vram_total` | GPU |
| Fan RPM | `s.fan_rpm` | Fan |
| Network download | `s.net_rx` | Down arrow |
| Network upload | `s.net_tx` | Up arrow |
| Load average | `s.load_avg` | Load |
| Swap | `s.swap` | Memory |

### Tables

- **Top processes** — Top 5 processes by CPU usage from `s.processes[]`, showing name, CPU%, and memory
- **Failed systemd units** — From `s.failed_units[]`, shown with a red warning icon when any are present
- **Active containers** — From `s.containers[]`, shown when Docker/Podman containers are running

### State fields read

`s.cpu`, `s.mem`, `s.disk`, `s.gpu_usage`, `s.gpu_temp`, `s.gpu_vram_used`, `s.gpu_vram_total`, `s.cpu_temp`, `s.fan_rpm`, `s.net_rx`, `s.net_tx`, `s.load_avg`, `s.swap`, `s.processes`, `s.failed_units`, `s.containers`

---

## 9. Media

**Panel ID:** `media`
**Opened by:** Clicking the now-playing ticker in the bar

MPRIS media player controls.

### What it shows

- **Player name** — Source player (e.g., "Spotify", "Firefox") from `s.media_player`
- **Album art** — Large artwork from `s.media_art_url`, or a music note placeholder
- **Track info** — Song title (`s.media_title`), artist (`s.media_artist`), album (`s.media_album`)
- **Transport controls** — Three circular buttons:

| Button | Action | Command |
|--------|--------|---------|
| Previous | Skip back | exec `playerctl previous` |
| Play/Pause | Toggle playback | exec `playerctl play-pause` |
| Next | Skip forward | exec `playerctl next` |

The play/pause button uses a larger accent-colored circle. The icon switches between play and pause based on `s.media_playing`.

### State fields read

`s.media_title`, `s.media_artist`, `s.media_album`, `s.media_art_url`, `s.media_playing`, `s.media_player`

---

## 10. Settings App

**Panel ID:** `config`
**Opened by:** Clicking the Config tile in Quick Settings

A full settings application with a sidebar navigation and 11 content pages. The sidebar is 140px wide with icon+label items. Clicking a sidebar item switches the visible page.

### Pages

#### Display

- Brightness slider (synced with `bri_set` command)
- Night Light toggle

**State fields:** `s.bright`

#### Sound

- Output volume slider with mute toggle
- Current output device name
- Quick-mute tip

**State fields:** `s.vol`, `s.muted`, `s.audio_device`

#### Network

- WiFi SSID and signal strength
- IP address
- VPN name and status
- Network speed (download/upload formatted)
- "Manage Networks" button (opens WiFi panel)

**State fields:** `s.wifi`, `s.wifi_signal`, `s.net_ip`, `s.vpn_name`, `s.net_rx`, `s.net_tx`

#### Bluetooth

- Power status (on/off)
- Toggle button
- Connected devices list

**State fields:** `s.bt_powered`, `s.bt_connected`

#### Power

- Battery gauge with percentage
- Charge status and power draw
- AC power indicator
- Power profile selector: Power Saver / Balanced / Performance (three clickable cards)

**State fields:** `s.bat`, `s.bat_status`, `s.power_draw`, `s.ac_power`, `s.power_profile`

#### Appearance

- Theme grid: 4-column grid of all 12 themes, each shown as a colored dot with the theme name. Click to switch theme instantly.
- Active theme has a highlighted ring.

**State fields:** `s.theme`

#### Notifications

- DND toggle
- Notification count
- "Dismiss All" button

**State fields:** `s.dnd`, `s.notif_count`

#### Storage

- Disk usage gauge
- Used / Total display
- Usage percentage

**State fields:** `s.disk`, `s.disk_used`, `s.disk_total`

#### About

- Username, hostname, OS, kernel version
- Desktop environment and compositor
- Uptime, CPU usage, memory usage
- Timezone, keyboard layout, load average

**State fields:** `s.username`, `s.hostname`, `s.kernel`, `s.compositor`, `s.uptime`, `s.cpu`, `s.mem`, `s.timezone`, `s.kb_layout`, `s.load_avg`

#### Gaming

- GameMode status
- GPU usage and temperature
- VRAM usage
- Gamescope status
- Discord status

**State fields:** `s.gamemode_active`, `s.gpu_usage`, `s.gpu_temp`, `s.gpu_vram_used`, `s.gpu_vram_total`, `s.gamescope_active`, `s.discord_active`

#### Containers

- List of running containers (Docker/Podman)
- Empty state when no containers are active

**State fields:** `s.containers`
