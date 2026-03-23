# Zenith v2 — Design Spec

## Context

Zenith is the demo/daily-driver shell for Pulpkit. v1 renders ~25 state fields across a bar + 5 popup panels. The POC now has 53 watchers feeding 186 state fields, a notification daemon, a DBus service, lock screen capabilities (session-lock protocol + PAM), and 9 Wayland protocol bindings. Zenith v2 showcases all of this while remaining clean and glanceable.

## Architecture Change: Configurable Popup Backdrop

### Problem
Current popups use a full-screen opaque `Layer::Overlay` backdrop that blanks the desktop. Users can't see their windows behind popups.

### Solution
Add `popup_backdrop` and `popup_dim_opacity` to `ThemeConfig` / `config.json`:

```json
{
  "popup_backdrop": "dim",
  "popup_dim_opacity": 0.25
}
```

**Rust changes** (`poc/src/main.rs`):
- Parse `popup_backdrop` from config: `"none"` | `"dim"` | `"opaque"` (default: `"dim"`)
- Parse `popup_dim_opacity` (default: `0.25`)
- For `"none"`: don't create backdrop_win at all. Dismiss via Escape key only (keyboard mode on popup).
- For `"dim"`: set backdrop background to `RGBA::new(0.0, 0.0, 0.0, opacity)`. Still catches clicks.
- For `"opaque"`: current behavior unchanged.

**Also**: Parse `ThemeConfig` from `config.json` — add fields to the existing struct, with defaults.

---

## Bar Enrichments

Keep current layout: `[workspaces | taskbar] [clock] [status | tray]`

### Additions to right status area (contextual — only show when relevant):
- **Media indicator**: artist — title (truncated 30ch) when `media_playing`. Click → toggle media popup.
- **Mic indicator**: mic icon when mic is unmuted/in-use. Click → toggle mute.
- **Notification badge**: dot/count on a bell icon when `notif_count > 0`. Click → notifications popup.
- **VPN indicator**: shield icon when `vpn_active`.
- **DND indicator**: moon icon when `dnd` active.
- **Gaming indicator**: gamepad icon when `gamemode_active`. Shows GPU temp next to it.
- **Caffeine indicator**: coffee icon when `caffeine_active`.

### New bar component files:
- `components/media.js` — now-playing ticker
- `components/indicators.js` — contextual status icons (vpn, dnd, gaming, caffeine, mic, notifications)

### Clock enrichment:
- Add date tooltip or click to open calendar popup

---

## Popup Panels

### Existing panels (enhanced):

**1. Settings** (`panels/settings.js`)
Current: 6 tiles + 2 sliders + battery row.
Enhanced:
- 8 tiles: WiFi, Bluetooth, DND, Night Light, Caffeine, VPN, Screen Share indicator, Gamemode
- 3 sliders: Volume, Brightness, Mic Volume
- Battery row: icon + percent + status + power draw (e.g., "12.3W") + time estimate
- AC indicator when plugged in
- Audio device selector (dropdown or row showing current device, click cycles)

**2. WiFi** (`panels/wifi.js`) — mostly unchanged, add signal bars visual

**3. Power** (`panels/power.js`) — add caffeine toggle row, session info enriched

**4. Launcher** (`panels/launcher.js`) — unchanged, already good

**5. Config/System** (`panels/config.js`)
Rename to "System" panel. Expand significantly:
- **Display**: brightness slider, outputs list (name, resolution, refresh), night light status
- **Audio**: volume slider, device name, audio device list with active indicator
- **Network**: wifi SSID, signal, IP, VPN name, download/upload speed (formatted)
- **System**: CPU %, memory %, swap, disk, load average, CPU temp, uptime, kernel
- **GPU** (if available): usage %, temp, VRAM used/total
- **Processes**: top 5 by CPU (name, cpu%, mem)

### New panels:

**6. Notifications** (`panels/notifications.js`)
- Title: "Notifications" with count + "Clear All" button
- List of notifications from `s.notifications[]`: app icon + app name + summary + body + timestamp
- Each notification: dismiss button (X), click to expand body
- Empty state: "No notifications"
- DND toggle at top

**7. Audio Mixer** (`panels/audio.js`)
- Title: "Audio"
- Output device selector (list of `s.audio_sinks`, active highlighted, click to switch)
- Input device selector (list of `s.audio_sources`)
- Per-app volume: list of `s.audio_streams[]` with name + slider + mute toggle
- Master volume + mic volume sliders at top

**8. Calendar/Weather** (`panels/calendar.js`)
- Title: "Calendar" with current date
- Weather row: temp + condition + icon + sunrise/sunset times
- Mini calendar grid (current month, highlight today)
- Upcoming events from `s.calendar_events[]` (if any)
- Timezone display

**9. Media** (`panels/media.js`)
- Title: player name (e.g., "Spotify")
- Album art (`s.media_art_url` as background or img)
- Track: title + artist + album
- Playback controls: prev / play-pause / next (via exec commands to playerctl)
- No progress bar needed (MPRIS doesn't give us position reactively)

**10. System Monitor** (`panels/monitor.js`)
- Full system overview for power users
- CPU: usage bar + temp + load averages
- Memory: usage bar + swap
- Disk: usage bar
- GPU: usage bar + temp + VRAM bar
- Fan RPM
- Network speed: ↓ rx/s ↑ tx/s (formatted human-readable)
- Top processes table (5 rows)
- Failed systemd units (if any, red warning)
- Active containers (if any)
- Active inhibitors (if any)

---

## Lock Screen

Uses ext-session-lock protocol (already bound in framework) + PAM verification.

For the POC (GTK4), the lock screen is simpler — use logind's `Lock` signal to trigger a lock screen popup that covers everything.

### Implementation:
- New file: `lock.html` — full-screen lock surface
- Design: centered clock (large), date, user avatar, password input
- On Enter: call PAM verify via a new IPC command `verify_password`
- On success: dismiss lock, call logind Unlock
- On failure: shake animation, "Incorrect password" message
- Background: blurred screenshot or solid color with clock

### New IPC command:
Add `"verify_password"` to `handle_command()` in main.rs — calls `pam::verify_password()`, returns result.

### Lock trigger:
The logind watcher already sets `session_locked: bool`. When this goes true:
- Show lock window (new Layer::Overlay surface covering everything)
- Hide bar and popups
- Capture keyboard

### Config:
```json
{
  "lock_style": "clock",
  "lock_background": "blur"
}
```

---

## File Structure

```
poc/shells/zenith/
  config.json
  bar.html
  popup.html
  lock.html                    ← NEW
  theme.css
  components/
    workspaces.js
    taskbar.js
    clock.js
    status.js
    tray.js
    media.js                   ← NEW (now-playing in bar)
    indicators.js              ← NEW (vpn, dnd, gaming, etc.)
  panels/
    settings.js                ← ENHANCED
    wifi.js                    ← MINOR UPDATES
    power.js                   ← MINOR UPDATES
    launcher.js                ← UNCHANGED
    config.js → system.js      ← RENAMED + EXPANDED
    notifications.js           ← NEW
    audio.js                   ← NEW
    calendar.js                ← NEW
    media.js                   ← NEW
    monitor.js                 ← NEW
```

---

## Verification

1. Build POC: `cargo build -p pulpkit-webshell-poc`
2. Run shell with zenith theme: `./target/debug/pulpkit-webshell-poc zenith`
3. Verify bar shows all contextual indicators when relevant
4. Open each popup, verify data populates from watchers
5. Test dim backdrop: desktop visible behind popup with 25% darken
6. Test lock screen: `loginctl lock-session`, verify password input works
7. Test notification daemon: `notify-send "Test" "Hello"` appears in notification panel
8. Test DBus service: `busctl --user call org.pulpkit.Shell /org/pulpkit/Shell org.pulpkit.Shell GetState` returns JSON
