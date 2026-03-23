# State Fields

The `FullState` struct contains all 187 fields that are serialized to JSON and pushed to WebViews every render cycle. This page documents every field, its type, which watcher provides it, and what it represents.

**Source**: `poc/src/state.rs`

---

## Audio

| Field | Type | Watcher | Description |
|-------|------|---------|-------------|
| `vol` | `u32` | `audio` | Current sink volume (0--100) |
| `muted` | `bool` | `audio` | Whether the default sink is muted |
| `audio_device` | `String` | `audio` | Description of the default audio sink |
| `audio_sinks` | `Vec<AudioDevice>` | `audio_devices` | All PulseAudio/PipeWire output devices |
| `audio_sources` | `Vec<AudioDevice>` | `audio_devices` | All PulseAudio/PipeWire input devices |
| `audio_streams` | `Vec<AudioStream>` | `audio_streams` | Active audio streams (per-app volume) |
| `mic_muted` | `bool` | `mic` | Whether the default source (microphone) is muted |
| `mic_volume` | `u32` | `mic` | Default source volume (0--100) |

## Display

| Field | Type | Watcher | Description |
|-------|------|---------|-------------|
| `bright` | `u32` | `brightness` | Screen brightness percentage (0--100) |
| `outputs` | `Vec<DisplayOutput>` | `outputs` | Connected display outputs with resolution, refresh, scale |
| `night_light_active` | `bool` | `night_light` | Whether a blue light filter (wlsunset/gammastep/redshift) is running |

## Battery / Power

| Field | Type | Watcher | Description |
|-------|------|---------|-------------|
| `bat` | `u32` | `upower` | Battery percentage (0--100) |
| `bat_status` | `String` | `upower` | Battery status: `"Charging"`, `"Discharging"`, `"Full"`, `"Empty"`, `"Pending charge"`, `"Pending discharge"`, `"Unknown"` |
| `has_bat` | `bool` | `upower` | Whether a battery is present (`/sys/class/power_supply/BAT0/capacity` exists) |
| `ac_plugged` | `bool` | `ac_power` | Whether AC power is connected |
| `power_profile` | `String` | `power_profiles` | Active power profile: `"power-saver"`, `"balanced"`, `"performance"` |
| `power_draw_watts` | `f32` | `power_draw` | Current power draw in watts (from sysfs `power_now` or V*I calculation) |

## System

| Field | Type | Watcher | Description |
|-------|------|---------|-------------|
| `cpu` | `u32` | background poller | CPU usage percentage (0--100), measured over 100ms sample |
| `mem` | `u32` | background poller | Memory usage percentage (0--100) |
| `disk_used` | `String` | background poller | Used disk space on `/` (human-readable, e.g. `"45G"`) |
| `disk_total` | `String` | background poller | Total disk space on `/` (human-readable, e.g. `"500G"`) |
| `disk_pct` | `u32` | background poller | Disk usage percentage (0--100) |
| `uptime` | `String` | background poller | System uptime (output of `uptime -p`) |
| `cpu_temp` | `u32` | `thermal` | CPU temperature in degrees Celsius |
| `fan_rpm` | `u32` | `fan` | Fan speed in RPM |
| `load_1` | `f32` | `load_avg` | 1-minute load average |
| `load_5` | `f32` | `load_avg` | 5-minute load average |
| `load_15` | `f32` | `load_avg` | 15-minute load average |
| `swap_used_mb` | `u32` | `swap` | Used swap in megabytes |
| `swap_total_mb` | `u32` | `swap` | Total swap in megabytes |
| `top_procs` | `Vec<ProcessInfo>` | `top_procs` | Top 5 processes by CPU usage |
| `compositor` | `String` | `compositor` | Detected compositor name: `"niri"`, `"hyprland"`, `"sway"`, etc. |

## Network

| Field | Type | Watcher | Description |
|-------|------|---------|-------------|
| `wifi` | `String` | `network` | Connected Wi-Fi SSID (empty if not connected) |
| `net_signal` | `u32` | `network` | Wi-Fi signal strength (0--100) |
| `net_ip` | `String` | `network` | Primary IP address |
| `net_rx_bytes_sec` | `u64` | `net_speed` | Network download speed in bytes/sec |
| `net_tx_bytes_sec` | `u64` | `net_speed` | Network upload speed in bytes/sec |
| `vpn_active` | `bool` | `vpn` | Whether a VPN (WireGuard or OpenVPN) is connected |
| `vpn_name` | `String` | `vpn` | Name of the active VPN connection |
| `wifi_nets` | `Vec<WifiNet>` | on-demand (`scan_wifi`) | Available Wi-Fi networks (populated when wifi popup opens) |

## Bluetooth

| Field | Type | Watcher | Description |
|-------|------|---------|-------------|
| `bt_powered` | `bool` | `bluetooth` | Whether the Bluetooth adapter is powered on |
| `bt_connected` | `Vec<BtDevice>` | `bluetooth` | Currently connected Bluetooth devices |

## Media

| Field | Type | Watcher | Description |
|-------|------|---------|-------------|
| `media_playing` | `bool` | `mpris` | Whether any MPRIS player is currently playing |
| `media_title` | `String` | `mpris` | Currently playing track title |
| `media_artist` | `String` | `mpris` | Currently playing track artist(s) |
| `media_album` | `String` | `mpris` | Currently playing track album |
| `media_art_url` | `String` | `mpris` | Album art URL (`file://` or `http://`) |
| `media_player` | `String` | `mpris` | Short name of the active media player (e.g. `"firefox"`, `"spotify"`) |

## Workspaces & Windows

| Field | Type | Watcher | Description |
|-------|------|---------|-------------|
| `ws` | `Vec<Workspace>` | `niri` | Workspace list with index and active flag |
| `windows` | `Vec<WindowInfo>` | `niri` | All open windows with id, title, app_id, focus state, icon path |
| `active_title` | `String` | `niri` | Title of the currently focused window |
| `active_app_id` | `String` | `niri` | App ID of the currently focused window |

## Session

| Field | Type | Watcher | Description |
|-------|------|---------|-------------|
| `session_locked` | `bool` | `logind` | Whether the session is locked (LockedHint) |
| `session_idle` | `bool` | `logind` | Whether the session is idle (IdleHint) |
| `preparing_sleep` | `bool` | `logind` | Whether the system is preparing to sleep |
| `inhibitors` | `Vec<InhibitorInfo>` | `inhibitors` | Active logind inhibitors (what, who, why) |

## Notifications

| Field | Type | Watcher | Description |
|-------|------|---------|-------------|
| `notifications` | `Vec<Notification>` | `notifications` | Active notifications received via the built-in notification daemon |
| `notif_count` | `u32` | `notifications` | Number of active notifications |
| `dnd` | `bool` | (updated via command) | Do Not Disturb mode |

## Gaming

| Field | Type | Watcher | Description |
|-------|------|---------|-------------|
| `gamemode_active` | `bool` | `gamemode` | Whether Feral GameMode has active clients |
| `gpu_usage` | `u32` | `gpu` | GPU busy percentage (0--100, from sysfs `gpu_busy_percent`) |
| `gpu_temp` | `u32` | `gpu` | GPU temperature in degrees Celsius |
| `vram_used_mb` | `u32` | `gpu` | Used VRAM in megabytes |
| `vram_total_mb` | `u32` | `gpu` | Total VRAM in megabytes |
| `gamescope_active` | `bool` | `gamescope` | Whether Gamescope is running |
| `discord_activity` | `String` | `discord` | Discord presence (empty or `"Discord running"`) |

## Clipboard

| Field | Type | Watcher | Description |
|-------|------|---------|-------------|
| `clipboard_text` | `String` | `clipboard` | Most recent clipboard text (truncated to 200 chars) |

## Keyboard

| Field | Type | Watcher | Description |
|-------|------|---------|-------------|
| `kb_layout` | `String` | `keyboard` | Active keyboard layout (e.g. `"us"`) |
| `kb_variant` | `String` | `keyboard` | Active keyboard variant (e.g. `"intl"`) |

## Input Method

| Field | Type | Watcher | Description |
|-------|------|---------|-------------|
| `im_active` | `bool` | `input_method` | Whether an input method (fcitx5 or ibus) is active |
| `im_name` | `String` | `input_method` | Current input method name |

## Removable Drives

| Field | Type | Watcher | Description |
|-------|------|---------|-------------|
| `drives` | `Vec<DriveInfo>` | `udisks` | Mounted removable drives |

## Systemd

| Field | Type | Watcher | Description |
|-------|------|---------|-------------|
| `failed_units` | `Vec<String>` | `systemd` | Names of failed systemd units |
| `failed_unit_count` | `u32` | `systemd` | Number of failed systemd units |
| `timers` | `Vec<TimerInfo>` | `systemd_timers` | Active systemd timers (up to 10) |

## System Tray

| Field | Type | Watcher | Description |
|-------|------|---------|-------------|
| `tray_items` | `Vec<TrayItem>` | `tray` | StatusNotifierItem tray entries |

## Applications

| Field | Type | Watcher | Description |
|-------|------|---------|-------------|
| `apps` | `Vec<AppEntry>` | on-demand (`scan_apps`) | Installed .desktop applications (populated when launcher popup opens) |

## UI State

| Field | Type | Watcher | Description |
|-------|------|---------|-------------|
| `popup` | `String` | AppState | Currently active popup name (empty = no popup) |
| `theme` | `String` | AppState | Active theme name |
| `custom` | `HashMap<String, Value>` | AppState / `dbus_service` | Arbitrary key-value store for theme-specific data |

## Static System Info

| Field | Type | Watcher | Description |
|-------|------|---------|-------------|
| `user` | `String` | main.rs (one-shot) | Current username |
| `host` | `String` | main.rs (one-shot) | Hostname |
| `kernel` | `String` | main.rs (one-shot) | Kernel version (`uname -r`) |
| `user_icon` | `String` | `user_info` | Path to the user's avatar icon (from AccountsService) |

## Polkit

| Field | Type | Watcher | Description |
|-------|------|---------|-------------|
| `polkit_pending` | `bool` | `polkit` | Whether a PolicyKit authentication request is pending |
| `polkit_message` | `String` | `polkit` | Message for the pending polkit request |

## Screen Sharing

| Field | Type | Watcher | Description |
|-------|------|---------|-------------|
| `screen_sharing` | `bool` | `screen_share` | Whether screen capture is active (PipeWire node check) |

## Trash

| Field | Type | Watcher | Description |
|-------|------|---------|-------------|
| `trash_count` | `u32` | `trash` | Number of items in `~/.local/share/Trash/files` |

## Package Updates

| Field | Type | Watcher | Description |
|-------|------|---------|-------------|
| `updates_available` | `u32` | `packagekit` | Number of available package updates |

## Timezone

| Field | Type | Watcher | Description |
|-------|------|---------|-------------|
| `timezone` | `String` | `timezone` | System timezone (e.g. `"America/New_York"`) |

## Journal

| Field | Type | Watcher | Description |
|-------|------|---------|-------------|
| `journal_errors` | `Vec<JournalEntry>` | `journal` | Recent journal entries at priority 3 (error) and above |

## SSH

| Field | Type | Watcher | Description |
|-------|------|---------|-------------|
| `ssh_sessions` | `u32` | `ssh_sessions` | Number of active inbound SSH connections on port 22 |

## Focus Tracking

| Field | Type | Watcher | Description |
|-------|------|---------|-------------|
| `focused_app_time_secs` | `u32` | `focus_tracker` | Seconds the current app has been focused |

## Caffeine

| Field | Type | Watcher | Description |
|-------|------|---------|-------------|
| `caffeine_active` | `bool` | `caffeine` | Whether caffeine mode is active (marker file at `$XDG_RUNTIME_DIR/pulpkit-caffeine`) |

## Weather

| Field | Type | Watcher | Description |
|-------|------|---------|-------------|
| `weather_temp` | `f32` | `weather` | Current temperature in degrees |
| `weather_condition` | `String` | `weather` | Weather condition text (e.g. `"Partly cloudy"`) |
| `weather_icon` | `String` | `weather` | Weather emoji icon from wttr.in |

## Sunrise / Sunset

| Field | Type | Watcher | Description |
|-------|------|---------|-------------|
| `sunrise` | `String` | `sunrise` | Sunrise time for today |
| `sunset` | `String` | `sunrise` | Sunset time for today |

## Containers

| Field | Type | Watcher | Description |
|-------|------|---------|-------------|
| `containers` | `Vec<ContainerInfo>` | `containers` | Running Docker/Podman containers |

## Calendar

| Field | Type | Watcher | Description |
|-------|------|---------|-------------|
| `calendar_events` | `Vec<CalendarEvent>` | `calendar` | Upcoming calendar events from local iCal files |

## Recent Files

| Field | Type | Watcher | Description |
|-------|------|---------|-------------|
| `recent_files` | `Vec<RecentFile>` | `recent_files` | Most recent 10 files from `recently-used.xbel` |

---

## Supporting Structs

### `Workspace`

| Field | Type | Description |
|-------|------|-------------|
| `idx` | `u32` | Workspace index |
| `active` | `bool` | Whether this workspace is focused |

### `WindowInfo`

| Field | Type | Description |
|-------|------|-------------|
| `id` | `u64` | Window ID |
| `title` | `String` | Window title |
| `app_id` | `String` | Application identifier |
| `focused` | `bool` | Whether this window is focused |
| `icon` | `String` | Resolved icon file path |

### `WifiNet`

| Field | Type | Description |
|-------|------|-------------|
| `ssid` | `String` | Network SSID |
| `signal` | `u32` | Signal strength (0--100) |
| `secure` | `bool` | Whether the network is encrypted |
| `active` | `bool` | Whether this is the connected network |

### `AppEntry`

| Field | Type | Description |
|-------|------|-------------|
| `name` | `String` | Application display name |
| `exec` | `String` | Exec command (without field codes) |
| `icon` | `String` | Resolved icon file path |

### `TrayItem`

| Field | Type | Description |
|-------|------|-------------|
| `id` | `String` | Item ID |
| `address` | `String` | DBus bus address |
| `title` | `String` | Item title |
| `icon` | `String` | Resolved icon file path |

### `BtDevice`

| Field | Type | Description |
|-------|------|-------------|
| `name` | `String` | Device name |
| `address` | `String` | MAC address |
| `connected` | `bool` | Connection state |
| `icon` | `String` | Device icon type (from bluez) |

### `AudioDevice`

| Field | Type | Description |
|-------|------|-------------|
| `name` | `String` | PulseAudio/PipeWire internal name |
| `description` | `String` | Human-readable device description |
| `active` | `bool` | Whether this is the default device |
| `volume` | `u32` | Device volume (0--100+) |
| `muted` | `bool` | Whether the device is muted |

### `AudioStream`

| Field | Type | Description |
|-------|------|-------------|
| `name` | `String` | Stream media name |
| `app_name` | `String` | Application name |
| `volume` | `u32` | Stream volume (0--100+) |
| `muted` | `bool` | Whether the stream is muted |
| `is_input` | `bool` | `true` for source-outputs, `false` for sink-inputs |

### `Notification`

| Field | Type | Description |
|-------|------|-------------|
| `id` | `u32` | Notification ID |
| `app_name` | `String` | Sending application name |
| `summary` | `String` | Notification title |
| `body` | `String` | Notification body text |
| `icon` | `String` | Icon name or path |
| `timestamp` | `u64` | Unix timestamp when received |

### `DriveInfo`

| Field | Type | Description |
|-------|------|-------------|
| `name` | `String` | Partition name (e.g. `"sdb1"`) |
| `mount_point` | `String` | Mount path |
| `size_bytes` | `u64` | Partition size in bytes |
| `device` | `String` | Device path (e.g. `"/dev/sdb1"`) |

### `InhibitorInfo`

| Field | Type | Description |
|-------|------|-------------|
| `who` | `String` | Who is inhibiting |
| `why` | `String` | Reason for inhibition |
| `what` | `String` | What is being inhibited (e.g. `"sleep:idle"`) |

### `DisplayOutput`

| Field | Type | Description |
|-------|------|-------------|
| `name` | `String` | Output name (e.g. `"DP-1"`) |
| `make` | `String` | Monitor manufacturer |
| `model` | `String` | Monitor model |
| `width` | `u32` | Horizontal resolution in pixels |
| `height` | `u32` | Vertical resolution in pixels |
| `refresh` | `f32` | Refresh rate in Hz |
| `scale` | `f32` | Output scale factor |
| `enabled` | `bool` | Whether the output is enabled |

### `ProcessInfo`

| Field | Type | Description |
|-------|------|-------------|
| `name` | `String` | Process command name |
| `pid` | `u32` | Process ID |
| `cpu_pct` | `f32` | CPU usage percentage |
| `mem_mb` | `u32` | RSS memory in megabytes |

### `ContainerInfo`

| Field | Type | Description |
|-------|------|-------------|
| `name` | `String` | Container name |
| `image` | `String` | Container image |
| `status` | `String` | Container status/state |
| `id` | `String` | Container ID (first 12 chars) |

### `TimerInfo`

| Field | Type | Description |
|-------|------|-------------|
| `name` | `String` | Unit that the timer activates |
| `next_trigger` | `String` | Next trigger time |
| `last_trigger` | `String` | Last trigger time |

### `JournalEntry`

| Field | Type | Description |
|-------|------|-------------|
| `unit` | `String` | Systemd unit name |
| `message` | `String` | Log message |
| `priority` | `u32` | Syslog priority (3 = error) |
| `timestamp` | `String` | Timestamp string |

### `CalendarEvent`

| Field | Type | Description |
|-------|------|-------------|
| `summary` | `String` | Event title |
| `start` | `String` | Start time (iCal DTSTART value) |
| `end` | `String` | End time (iCal DTEND value) |
| `location` | `String` | Event location |

### `RecentFile`

| Field | Type | Description |
|-------|------|-------------|
| `name` | `String` | File name (percent-decoded) |
| `uri` | `String` | Full file URI |
| `mime_type` | `String` | MIME type |
| `timestamp` | `u64` | Sortable timestamp from the `modified` attribute |

---

## AppState (internal, not serialized directly)

The `AppState` struct lives on the main GTK thread in a `Rc<RefCell>` and holds fields that are managed by the command handler rather than watchers:

| Field | Type | Description |
|-------|------|-------------|
| `popup` | `String` | Currently active popup |
| `theme` | `String` | Active theme |
| `wifi_nets` | `Vec<WifiNet>` | Scanned Wi-Fi networks |
| `apps` | `Vec<AppEntry>` | Scanned desktop applications |
| `tray_items` | `Vec<TrayItem>` | Copied from the tray watcher each frame |
| `custom` | `HashMap<String, Value>` | Custom key-value store |
| `dirty` | `Arc<AtomicBool>` | Shared dirty flag |
| `tray_activate_tx` | `Option<Sender>` | Channel to send tray activation events to the tray watcher |
| `clear_notifications` | `bool` | Flag to clear all notifications on next render cycle |

These fields are merged into `FullState` during serialization in the 80ms render loop.
