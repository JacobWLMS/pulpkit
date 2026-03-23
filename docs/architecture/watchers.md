# Watchers

Pulpkit uses 53 watcher modules to collect system state. Each watcher runs in its own `std::thread`, holds `Arc<Mutex<FullState>>` and `Arc<AtomicBool>` (dirty flag), and updates fields when data changes.

## Watcher Table

### Audio (4 watchers)

| Watcher | Module | Mechanism | Event-driven? | Interval | State fields |
|---------|--------|-----------|:---:|----------|--------------|
| **audio** | `watchers::audio` | `pactl subscribe` stream | Yes | -- | `vol`, `muted`, `audio_device` |
| **audio_devices** | `watchers::audio_devices` | `pactl subscribe` stream | Yes | -- | `audio_sinks`, `audio_sources` |
| **audio_streams** | `watchers::audio_streams` | `pactl subscribe` stream | Yes | -- | `audio_streams` |
| **mic** | `watchers::mic` | `pactl subscribe` stream | Yes | -- | `mic_muted`, `mic_volume` |

### Network (3 watchers)

| Watcher | Module | Mechanism | Event-driven? | Interval | State fields |
|---------|--------|-----------|:---:|----------|--------------|
| **network** | `watchers::network` | DBus signals (`org.freedesktop.NetworkManager`) | Yes | -- | `wifi`, `net_signal`, `net_ip` |
| **net_speed** | `watchers::net_speed` | sysfs poll (`/proc/net/dev`) | No | 2s | `net_rx_bytes_sec`, `net_tx_bytes_sec` |
| **vpn** | `watchers::vpn` | DBus signals (NetworkManager) + fallback `ip link` | Yes | -- | `vpn_active`, `vpn_name` |

### Bluetooth (1 watcher)

| Watcher | Module | Mechanism | Event-driven? | Interval | State fields |
|---------|--------|-----------|:---:|----------|--------------|
| **bluetooth** | `watchers::bluetooth` | DBus signals (`org.bluez`) | Yes | -- | `bt_powered`, `bt_connected` |

### Media (1 watcher)

| Watcher | Module | Mechanism | Event-driven? | Interval | State fields |
|---------|--------|-----------|:---:|----------|--------------|
| **mpris** | `watchers::mpris` | DBus signals (session bus, MPRIS2) | Yes | -- | `media_playing`, `media_title`, `media_artist`, `media_album`, `media_art_url`, `media_player` |

### Battery / Power (4 watchers)

| Watcher | Module | Mechanism | Event-driven? | Interval | State fields |
|---------|--------|-----------|:---:|----------|--------------|
| **upower** | `watchers::upower` | DBus PropertiesChanged (`org.freedesktop.UPower`) | Yes | -- | `bat`, `bat_status`, `has_bat` |
| **ac_power** | `watchers::ac_power` | `udevadm monitor --subsystem-match=power_supply` | Yes | -- | `ac_plugged` |
| **power_profiles** | `watchers::power_profiles` | DBus PropertiesChanged (`net.hadess.PowerProfiles`) | Yes | -- | `power_profile` |
| **power_draw** | `watchers::power_draw` | sysfs poll (`/sys/class/power_supply/BAT0/power_now`) | No | 3s | `power_draw_watts` |

### Display (3 watchers)

| Watcher | Module | Mechanism | Event-driven? | Interval | State fields |
|---------|--------|-----------|:---:|----------|--------------|
| **brightness** | `watchers::brightness` | `udevadm monitor --subsystem-match=backlight` | Yes | -- | `bright` |
| **outputs** | `watchers::outputs` | `niri msg -j outputs` poll | No | 10s | `outputs` |
| **night_light** | `watchers::night_light` | `pgrep` poll (wlsunset/gammastep/redshift) | No | 5s | `night_light_active` |

### System (8 watchers)

| Watcher | Module | Mechanism | Event-driven? | Interval | State fields |
|---------|--------|-----------|:---:|----------|--------------|
| **thermal** | `watchers::thermal` | sysfs poll (`/sys/class/thermal/thermal_zone*/temp`) | No | 3s | `cpu_temp` |
| **fan** | `watchers::fan` | sysfs poll (`/sys/class/hwmon/*/fan1_input`) | No | 5s | `fan_rpm` |
| **load_avg** | `watchers::load_avg` | `/proc/loadavg` poll | No | 5s | `load_1`, `load_5`, `load_15` |
| **swap** | `watchers::swap` | `/proc/meminfo` poll | No | 5s | `swap_used_mb`, `swap_total_mb` |
| **top_procs** | `watchers::top_procs` | `ps aux --sort=-%cpu` poll | No | 5s | `top_procs` |
| **compositor** | `watchers::compositor` | Env var detection (one-shot) | -- | -- | `compositor` |
| **user_info** | `watchers::user_info` | DBus one-shot (`org.freedesktop.Accounts`) | -- | -- | `user_icon` |
| **keyboard** | `watchers::keyboard` | `setxkbmap -query` poll | No | 3s | `kb_layout`, `kb_variant` |

### Session (2 watchers)

| Watcher | Module | Mechanism | Event-driven? | Interval | State fields |
|---------|--------|-----------|:---:|----------|--------------|
| **logind** | `watchers::logind` | DBus signals (`org.freedesktop.login1`) | Yes | -- | `session_locked`, `session_idle`, `preparing_sleep` |
| **inhibitors** | `watchers::inhibitors` | DBus poll (`login1.Manager.ListInhibitors`) | No | 10s | `inhibitors` |

### Gaming (4 watchers)

| Watcher | Module | Mechanism | Event-driven? | Interval | State fields |
|---------|--------|-----------|:---:|----------|--------------|
| **gamemode** | `watchers::gamemode` | DBus signals (`com.feralinteractive.GameMode`) | Yes | -- | `gamemode_active` |
| **gpu** | `watchers::gpu` | sysfs poll (`/sys/class/drm/card*/device/`) | No | 2s | `gpu_usage`, `gpu_temp`, `vram_used_mb`, `vram_total_mb` |
| **gamescope** | `watchers::gamescope` | `pgrep` + env var poll | No | 5s | `gamescope_active` |
| **discord** | `watchers::discord` | File existence poll (`$XDG_RUNTIME_DIR/discord-ipc-*`) | No | 5s | `discord_activity` |

### Desktop (5 watchers)

| Watcher | Module | Mechanism | Event-driven? | Interval | State fields |
|---------|--------|-----------|:---:|----------|--------------|
| **niri** | `watchers::niri` | `niri msg event-stream` (stdout stream) | Yes | -- | `ws`, `windows`, `active_title`, `active_app_id` |
| **clipboard** | `watchers::clipboard` | `wl-paste --watch` (stdout stream) | Yes | -- | `clipboard_text` |
| **tray** | `watchers::tray` | StatusNotifierWatcher (async, tokio) | Yes | -- | `tray_items` (via separate `Arc<Mutex<Vec>>`) |
| **trash** | `watchers::trash` | `inotifywait` on `~/.local/share/Trash/files` | Yes | -- | `trash_count` |
| **polkit** | `watchers::polkit` | DBus signals (`org.freedesktop.PolicyKit1`) | Yes | -- | `polkit_pending`, `polkit_message` |

### Monitoring (6 watchers)

| Watcher | Module | Mechanism | Event-driven? | Interval | State fields |
|---------|--------|-----------|:---:|----------|--------------|
| **systemd** | `watchers::systemd` | DBus signals (`org.freedesktop.systemd1`) | Yes | -- | `failed_units`, `failed_unit_count` |
| **systemd_timers** | `watchers::systemd_timers` | `systemctl list-timers` poll | No | 60s | `timers` |
| **journal** | `watchers::journal` | `journalctl -p 3 -n 10` poll | No | 30s | `journal_errors` |
| **containers** | `watchers::containers` | `podman ps` / `docker ps` poll | No | 10s | `containers` |
| **ssh_sessions** | `watchers::ssh_sessions` | `ss -tn sport = :22` poll | No | 10s | `ssh_sessions` |
| **screen_share** | `watchers::screen_share` | `pw-cli ls Node` poll | No | 3s | `screen_sharing` |

### Services (3 watchers)

| Watcher | Module | Mechanism | Event-driven? | Interval | State fields |
|---------|--------|-----------|:---:|----------|--------------|
| **notifications** | `watchers::notifications` | DBus service (`org.freedesktop.Notifications`, tokio) | Yes | -- | `notifications`, `notif_count` |
| **dbus_service** | `watchers::dbus_service` | DBus service (`org.pulpkit.Shell`, tokio) | Yes | -- | (API-only, no direct fields) |
| **input_method** | `watchers::input_method` | DBus signals (fcitx5 or ibus) | Yes | -- | `im_active`, `im_name` |

### Other (9 watchers)

| Watcher | Module | Mechanism | Event-driven? | Interval | State fields |
|---------|--------|-----------|:---:|----------|--------------|
| **weather** | `watchers::weather` | `curl wttr.in` poll | No | 900s | `weather_temp`, `weather_condition`, `weather_icon` |
| **sunrise** | `watchers::sunrise` | `curl wttr.in` poll | No | 3600s | `sunrise`, `sunset` |
| **calendar** | `watchers::calendar` | iCal file scan poll | No | 300s | `calendar_events` |
| **recent_files** | `watchers::recent_files` | `recently-used.xbel` file poll | No | 30s | `recent_files` |
| **packagekit** | `watchers::packagekit` | `checkupdates` / `apt` / `dnf` poll | No | 1800s | `updates_available` |
| **timezone** | `watchers::timezone` | DBus PropertiesChanged (`org.freedesktop.timedate1`) | Yes | -- | `timezone` |
| **focus_tracker** | `watchers::focus_tracker` | Internal timer (reads `active_app_id` from state) | No | 1s | `focused_app_time_secs` |
| **caffeine** | `watchers::caffeine` | File existence poll (`$XDG_RUNTIME_DIR/pulpkit-caffeine`) | No | 5s | `caffeine_active` |
| **udisks** | `watchers::udisks` | DBus signals (`org.freedesktop.UDisks2`) | Yes | -- | `drives` |

### Background Poller (in main.rs)

In addition to the watcher modules, a single background thread polls the remaining system metrics:

| Fields | Mechanism | Interval |
|--------|-----------|----------|
| `cpu`, `mem`, `disk_used`, `disk_total`, `disk_pct`, `uptime` | `/proc/stat`, `/proc/meminfo`, `df`, `uptime` | 3s |

## Mechanism Summary

| Mechanism | Count | Description |
|-----------|-------|-------------|
| **DBus signals** | 17 | Subscribe to `PropertiesChanged` or service-specific signals via `zbus::blocking::MessageIterator` |
| **Stream (stdout)** | 6 | Spawn child process, read lines from stdout (`pactl subscribe`, `niri msg event-stream`, `wl-paste --watch`, `udevadm monitor`, `inotifywait`) |
| **sysfs/procfs poll** | 10 | Read files from `/sys/` or `/proc/` on a timer |
| **Shell command poll** | 11 | Run a shell command and parse output on a timer |
| **DBus service** | 3 | Expose a DBus interface using tokio + `zbus::ConnectionBuilder` |
| **File existence poll** | 3 | Check if a file/socket exists on disk |
| **One-shot** | 2 | Read once at startup, never poll again |
| **Internal** | 1 | Reads other state fields on a 1s timer |

## How to Add a New Watcher

### 1. Create the module

Create `poc/src/watchers/my_watcher.rs`:

```rust
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use crate::state::FullState;

pub fn start_my_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        // Initial read
        let value = read_my_data();
        if let Ok(mut s) = state.lock() {
            s.my_field = value;
        }
        dirty.store(true, Ordering::Relaxed);

        // Option A: Event-driven (DBus signals)
        // let conn = zbus::blocking::Connection::system().unwrap();
        // let rule = zbus::MatchRule::builder()...build();
        // let iter = zbus::blocking::MessageIterator::for_match_rule(rule, &conn, Some(16)).unwrap();
        // for _msg in iter { update_state(&state, &dirty); }

        // Option B: Poll loop
        // loop {
        //     let value = read_my_data();
        //     if let Ok(mut s) = state.lock() { s.my_field = value; }
        //     dirty.store(true, Ordering::Relaxed);
        //     std::thread::sleep(std::time::Duration::from_secs(5));
        // }

        // Option C: Stream (stdout from child process)
        // let mut child = Command::new("my-tool").arg("--watch")
        //     .stdout(Stdio::piped()).spawn().unwrap();
        // let reader = BufReader::new(child.stdout.take().unwrap());
        // for line in reader.lines().flatten() { update_state(&state, &dirty); }
    });
}
```

### 2. Add state fields

Add your fields to `FullState` in `poc/src/state.rs`:

```rust
pub struct FullState {
    // ... existing fields ...
    pub my_field: String,
}
```

### 3. Register the module

Add to `poc/src/watchers/mod.rs`:

```rust
pub mod my_watcher;
```

### 4. Start the watcher

Add to the watcher startup block in `poc/src/main.rs`:

```rust
watchers::my_watcher::start_my_watcher(polled_state.clone(), dirty_flag.clone());
```

### Key patterns

- **Always do an initial read** before entering the event loop so state is populated before the first render.
- **Prefer event-driven** mechanisms (DBus signals, streams) over polling.
- **Use `zbus::blocking`** for watchers that only read properties -- no need for tokio.
- **Use tokio** only when you need to _serve_ a DBus interface (notifications, shell service, tray).
- **Set the dirty flag** after every state update so the render loop picks up changes.
- **Gracefully handle missing services** -- log a warning and return if a DBus service is unavailable.
