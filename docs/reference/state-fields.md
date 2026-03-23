# State Fields Reference

Complete reference for every field in the state object pushed to `updateState(s)`.
State is pushed approximately every 80ms after any reactive change. All fields are
always present (strings default to `""`, numbers to `0`, booleans to `false`,
arrays to `[]`).

## Audio

| Field | Type | Description | Source |
|---|---|---|---|
| `vol` | `number` (0-100) | Audio volume | PipeWire/PulseAudio watcher |
| `muted` | `boolean` | Audio muted | PipeWire/PulseAudio watcher |
| `audio_device` | `string` | Active output device name | PipeWire/PulseAudio watcher |
| `mic_volume` | `number` (0-100) | Microphone volume | Mic watcher |
| `mic_muted` | `boolean` | Microphone muted | Mic watcher |
| `audio_sinks` | `AudioDevice[]` | Output devices | Audio devices watcher |
| `audio_sources` | `AudioDevice[]` | Input devices | Audio devices watcher |
| `audio_streams` | `AudioStream[]` | Active PipeWire streams | Audio streams watcher |

### AudioDevice

```ts
{
  name: string,         // PipeWire node name
  description: string,  // Human-readable description
  active: boolean,      // Currently selected
  volume: number,       // 0-100
  muted: boolean
}
```

### AudioStream

```ts
{
  name: string,      // Stream name
  app_name: string,  // Application name
  volume: number,    // 0-100
  muted: boolean,
  is_input: boolean  // true = microphone/input, false = speaker/output
}
```

## Battery & Power

| Field | Type | Description | Source |
|---|---|---|---|
| `bat` | `number` (0-100) | Battery percentage | UPower watcher |
| `bat_status` | `string` | `"Charging"` / `"Discharging"` / `"Full"` / `"Not charging"` | UPower watcher |
| `has_bat` | `boolean` | Whether device has a battery | Detected at startup |
| `ac_plugged` | `boolean` | AC power supply connected | AC power watcher |
| `power_profile` | `string` | `"balanced"` / `"performance"` / `"power-saver"` | power-profiles-daemon watcher |
| `power_draw_watts` | `number` | Current power draw in watts | Power draw watcher |

## Bluetooth

| Field | Type | Description | Source |
|---|---|---|---|
| `bt_powered` | `boolean` | Bluetooth adapter powered on | Bluetooth watcher |
| `bt_connected` | `BtDevice[]` | Paired/connected devices | Bluetooth watcher |

### BtDevice

```ts
{
  name: string,       // Device name
  address: string,    // MAC address
  connected: boolean, // Currently connected
  icon: string        // Device icon name
}
```

## Clipboard

| Field | Type | Description | Source |
|---|---|---|---|
| `clipboard_text` | `string` | Current clipboard text content | Clipboard watcher |

## Display

| Field | Type | Description | Source |
|---|---|---|---|
| `bright` | `number` (0-100) | Screen brightness | Brightness watcher |
| `night_light_active` | `boolean` | Night light (wlsunset) enabled | Night light watcher |
| `screen_sharing` | `boolean` | Screen sharing active | Screen share watcher |
| `outputs` | `DisplayOutput[]` | Connected display outputs | Outputs watcher |

### DisplayOutput

```ts
{
  name: string,     // Output name (e.g. "eDP-1")
  make: string,     // Manufacturer
  model: string,    // Model name
  width: number,    // Resolution width (px)
  height: number,   // Resolution height (px)
  refresh: number,  // Refresh rate (Hz, float)
  scale: number,    // Scale factor (float)
  enabled: boolean  // Output enabled
}
```

## Gaming

| Field | Type | Description | Source |
|---|---|---|---|
| `gamemode_active` | `boolean` | GameMode enabled | GameMode watcher |
| `gpu_usage` | `number` (0-100) | GPU usage percentage | GPU watcher |
| `gpu_temp` | `number` | GPU temperature (degrees C) | GPU watcher |
| `vram_used_mb` | `number` | VRAM used in MB | GPU watcher |
| `vram_total_mb` | `number` | VRAM total in MB | GPU watcher |
| `gamescope_active` | `boolean` | Gamescope session active | Gamescope watcher |
| `discord_activity` | `string` | Current Discord Rich Presence activity | Discord watcher |

## Keyboard & Input

| Field | Type | Description | Source |
|---|---|---|---|
| `kb_layout` | `string` | Keyboard layout (e.g. `"us"`) | Keyboard watcher |
| `kb_variant` | `string` | Keyboard variant | Keyboard watcher |
| `im_active` | `boolean` | Input method (Fcitx/IBus) active | Input method watcher |
| `im_name` | `string` | Input method name | Input method watcher |

## MPRIS Media

| Field | Type | Description | Source |
|---|---|---|---|
| `media_playing` | `boolean` | Media currently playing | MPRIS watcher |
| `media_title` | `string` | Track title | MPRIS watcher |
| `media_artist` | `string` | Track artist | MPRIS watcher |
| `media_album` | `string` | Track album | MPRIS watcher |
| `media_art_url` | `string` | Album art URL | MPRIS watcher |
| `media_player` | `string` | Player name (e.g. `"spotify"`) | MPRIS watcher |

## Network

| Field | Type | Description | Source |
|---|---|---|---|
| `wifi` | `string` | Connected SSID or `""` | Network watcher |
| `net_signal` | `number` (0-100) | WiFi signal strength | Network watcher |
| `net_ip` | `string` | IP address | Network watcher |
| `net_rx_bytes_sec` | `number` | Network download bytes/sec | Net speed watcher |
| `net_tx_bytes_sec` | `number` | Network upload bytes/sec | Net speed watcher |
| `wifi_nets` | `WifiNet[]` | Available WiFi networks (populated when wifi popup opens) | On-demand scan |
| `vpn_active` | `boolean` | VPN connection active | VPN watcher |
| `vpn_name` | `string` | Active VPN connection name | VPN watcher |

### WifiNet

```ts
{
  ssid: string,    // Network name
  signal: number,  // 0-100 signal strength
  secure: boolean, // WPA/WPA2 protected
  active: boolean  // Currently connected
}
```

## Notifications

| Field | Type | Description | Source |
|---|---|---|---|
| `notif_count` | `number` | Number of pending notifications | Notification daemon |
| `dnd` | `boolean` | Do Not Disturb mode active | Notification daemon |
| `notifications` | `Notification[]` | Notification list | Notification daemon |

### Notification

```ts
{
  id: number,        // Unique notification ID
  app_name: string,  // Sending application name
  summary: string,   // Notification title
  body: string,      // Notification body text
  icon: string,      // Icon path or name
  timestamp: number  // Unix timestamp (seconds)
}
```

## Removable Drives

| Field | Type | Description | Source |
|---|---|---|---|
| `drives` | `DriveInfo[]` | Mounted removable drives | UDisks watcher |

### DriveInfo

```ts
{
  name: string,         // Drive label
  mount_point: string,  // Mount path
  size_bytes: number,   // Total size in bytes
  device: string        // Device path (e.g. "/dev/sdb1")
}
```

## Session

| Field | Type | Description | Source |
|---|---|---|---|
| `session_locked` | `boolean` | Session is locked | Logind watcher |
| `session_idle` | `boolean` | Session is idle | Logind watcher |
| `preparing_sleep` | `boolean` | System preparing to sleep | Logind watcher |
| `caffeine_active` | `boolean` | Manual idle inhibit active | Caffeine watcher |

## System

| Field | Type | Description | Source |
|---|---|---|---|
| `cpu` | `number` (0-100) | CPU usage percentage | Background poller (3s) |
| `mem` | `number` (0-100) | RAM usage percentage | Background poller (3s) |
| `cpu_temp` | `number` | CPU temperature (degrees C) | Thermal watcher |
| `disk_used` | `string` | Disk used (e.g. `"171G"`) | Background poller (3s) |
| `disk_total` | `string` | Disk total (e.g. `"248G"`) | Background poller (3s) |
| `disk_pct` | `number` (0-100) | Disk usage percentage | Background poller (3s) |
| `swap_used_mb` | `number` | Swap used in MB | Swap watcher |
| `swap_total_mb` | `number` | Swap total in MB | Swap watcher |
| `load_1` | `number` | 1-minute load average | Load avg watcher |
| `load_5` | `number` | 5-minute load average | Load avg watcher |
| `load_15` | `number` | 15-minute load average | Load avg watcher |
| `uptime` | `string` | System uptime (e.g. `"3 hours, 12 min"`) | Background poller (3s) |
| `timezone` | `string` | System timezone | Timezone watcher |
| `compositor` | `string` | Compositor name (e.g. `"niri"`, `"sway"`, `"hyprland"`) | Compositor watcher |
| `fan_rpm` | `number` | Fan speed in RPM | Fan watcher |
| `top_procs` | `ProcessInfo[]` | Top processes by CPU usage | Top procs watcher |

### ProcessInfo

```ts
{
  name: string,    // Process name
  pid: number,     // Process ID
  cpu_pct: number, // CPU usage percentage (float)
  mem_mb: number   // Memory usage in MB
}
```

## Systemd

| Field | Type | Description | Source |
|---|---|---|---|
| `failed_units` | `string[]` | List of failed systemd unit names | Systemd watcher |
| `failed_unit_count` | `number` | Number of failed systemd units | Systemd watcher |
| `timers` | `TimerInfo[]` | Active systemd timers | Systemd timers watcher |
| `journal_errors` | `JournalEntry[]` | Recent journal error entries | Journal watcher |

### TimerInfo

```ts
{
  name: string,         // Timer unit name
  next_trigger: string, // Next trigger time
  last_trigger: string  // Last trigger time
}
```

### JournalEntry

```ts
{
  unit: string,      // Systemd unit name
  message: string,   // Log message
  priority: number,  // Syslog priority level
  timestamp: string  // ISO timestamp
}
```

## Applications & Tray

| Field | Type | Description | Source |
|---|---|---|---|
| `apps` | `AppEntry[]` | Installed applications (populated when launcher opens) | On-demand scan |
| `tray_items` | `TrayItem[]` | System tray items | Tray watcher |

### AppEntry

```ts
{
  name: string,  // Application name
  exec: string,  // Exec command
  icon: string   // Icon path or name
}
```

### TrayItem

```ts
{
  id: string,      // Item identifier
  address: string, // DBus address (used for tray_activate command)
  title: string,   // Display title
  icon: string     // Icon path or name
}
```

## Calendar & Weather

| Field | Type | Description | Source |
|---|---|---|---|
| `calendar_events` | `CalendarEvent[]` | Upcoming iCal events | Calendar watcher |
| `weather_temp` | `number` | Temperature (float) | Weather watcher |
| `weather_condition` | `string` | Condition description | Weather watcher |
| `weather_icon` | `string` | Weather icon name | Weather watcher |
| `sunrise` | `string` | Sunrise time | Sunrise watcher |
| `sunset` | `string` | Sunset time | Sunrise watcher |

### CalendarEvent

```ts
{
  summary: string,  // Event title
  start: string,    // Start time
  end: string,      // End time
  location: string  // Event location
}
```

## Containers

| Field | Type | Description | Source |
|---|---|---|---|
| `containers` | `ContainerInfo[]` | Docker/Podman containers | Containers watcher |

### ContainerInfo

```ts
{
  name: string,   // Container name
  image: string,  // Image name
  status: string, // Container status
  id: string      // Container ID
}
```

## Idle Inhibitors

| Field | Type | Description | Source |
|---|---|---|---|
| `inhibitors` | `InhibitorInfo[]` | Active idle inhibitors | Inhibitor watcher |

### InhibitorInfo

```ts
{
  who: string,  // Application name
  why: string,  // Reason for inhibiting
  what: string  // What is being inhibited
}
```

## Miscellaneous

| Field | Type | Description | Source |
|---|---|---|---|
| `updates_available` | `number` | Available package updates | PackageKit watcher |
| `trash_count` | `number` | Items in trash | Trash watcher |
| `ssh_sessions` | `number` | Active SSH sessions | SSH sessions watcher |
| `focused_app_time_secs` | `number` | Seconds focused app has been active | Focus tracker |
| `recent_files` | `RecentFile[]` | Recently opened files | Recent files watcher |

### RecentFile

```ts
{
  name: string,      // File name
  uri: string,       // File URI
  mime_type: string,  // MIME type
  timestamp: number  // Unix timestamp
}
```

## Polkit

| Field | Type | Description | Source |
|---|---|---|---|
| `polkit_pending` | `boolean` | Polkit authorization pending | Polkit watcher |
| `polkit_message` | `string` | Polkit authorization message | Polkit watcher |

## UI State

| Field | Type | Description | Source |
|---|---|---|---|
| `popup` | `string` | Currently open popup panel name, or `""` | Shell command handler |
| `theme` | `string` | Active color theme name | Shell command handler |
| `custom` | `object` | Arbitrary key-value store | `set_custom` command |

## User & Host

| Field | Type | Description | Source |
|---|---|---|---|
| `user` | `string` | Username | Detected at startup |
| `user_icon` | `string` | User avatar path | User info watcher |
| `host` | `string` | Hostname | Detected at startup |
| `kernel` | `string` | Kernel version | Detected at startup |

## Workspaces & Windows

| Field | Type | Description | Source |
|---|---|---|---|
| `ws` | `Workspace[]` | Workspace list | Niri/compositor watcher |
| `windows` | `WindowInfo[]` | Running windows | Niri/compositor watcher |
| `active_title` | `string` | Focused window title | Niri/compositor watcher |
| `active_app_id` | `string` | Focused window app_id | Niri/compositor watcher |

### Workspace

```ts
{
  idx: number,    // Workspace index
  active: boolean // Currently active
}
```

### WindowInfo

```ts
{
  id: number,       // Window ID
  title: string,    // Window title
  app_id: string,   // Application identifier
  focused: boolean, // Currently focused
  icon: string      // Application icon path
}
```
