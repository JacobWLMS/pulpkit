# Zenith v2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Upgrade Zenith shell with configurable dim popups, enriched panels showing all 186 state fields, new bar indicators, and a lock screen.

**Architecture:** Rust backend changes for backdrop config + lock IPC, then pure HTML/JS/CSS changes in the shell theme directory. Existing 10-panel popup structure is kept and enhanced — no panels removed.

**Tech Stack:** Rust (POC main.rs), HTML/CSS/JS (Zenith shell), GTK4 layer-shell, Nerd Font icons

**Spec:** `docs/superpowers/specs/2026-03-23-zenith-v2-design.md`

---

## File Map

### Rust (modify)
- `poc/src/main.rs` — backdrop config parsing, dim RGBA, lock screen window, verify_password IPC command

### Shell (modify)
- `poc/shells/zenith/config.json` — add popup_backdrop, popup_dim_opacity
- `poc/shells/zenith/bar.html` — add indicator containers in status area, import new components
- `poc/shells/zenith/popup.html` — enhance existing panels with new state fields, add audio mixer panel

### Shell (create)
- `poc/shells/zenith/lock.html` — lock screen surface
- `poc/shells/zenith/components/indicators.js` — VPN, DND, gaming, caffeine, mic indicators

---

## Task 1: Configurable Backdrop (Rust)

**Files:**
- Modify: `poc/src/main.rs`

- [ ] **Step 1: Add backdrop config fields to ThemeConfig**

In main.rs, add to `ThemeConfig`:
```rust
#[derive(serde::Deserialize)]
struct ThemeConfig {
    #[serde(default = "default_bar_height")]
    bar_height: i32,
    #[serde(default = "default_popup_width")]
    popup_width: i32,
    #[serde(default = "default_popup_height")]
    popup_height: i32,
    #[serde(default)]
    bar_position: String,
    #[serde(default = "default_popup_backdrop")]
    popup_backdrop: String,  // "none" | "dim" | "opaque"
    #[serde(default = "default_popup_dim_opacity")]
    popup_dim_opacity: f32,
}

fn default_popup_backdrop() -> String { "dim".into() }
fn default_popup_dim_opacity() -> f32 { 0.25 }
```

- [ ] **Step 2: Apply backdrop opacity based on config**

In the `connect_activate` closure, after creating backdrop_win, set its background:
```rust
// After backdrop_win creation, before set_visible(false):
match theme_config.popup_backdrop.as_str() {
    "none" => {
        // Don't show backdrop at all — handled in render loop
    }
    "dim" => {
        let opacity = theme_config.popup_dim_opacity;
        backdrop_win.set_background_color(&gtk4::gdk::RGBA::new(0.0, 0.0, 0.0, opacity));
    }
    _ => {
        // "opaque" — default solid black background (current behavior via CSS)
    }
}
```

Wait — `backdrop_win` is a `gtk4::Window`, not a WebView. GTK4 windows don't have `set_background_color`. We need to use CSS to set the backdrop opacity.

Correct approach: use a CSS provider on the backdrop window:
```rust
let css = gtk4::CssProvider::new();
let opacity = theme_config.popup_dim_opacity;
let css_str = match theme_config.popup_backdrop.as_str() {
    "none" => "window { background: transparent; }".to_string(),
    "dim" => format!("window {{ background: rgba(0,0,0,{opacity}); }}"),
    _ => "window { background: rgba(0,0,0,0.95); }".to_string(),
};
css.load_from_string(&css_str);
backdrop_win.style_context().add_provider(&css, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
```

Actually in GTK4, `style_context()` is deprecated. Use `gtk4::style_context_add_provider_for_display` instead:
```rust
let display = gtk4::gdk::Display::default().unwrap();
gtk4::style_context_add_provider_for_display(&display, &css, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
```

But that affects ALL windows. Better: use inline CSS on just the backdrop. The simplest GTK4 approach is to add a CSS class and provider scoped to the widget.

Simplest correct approach for GTK4:
```rust
let css = gtk4::CssProvider::new();
let opacity = theme_config.popup_dim_opacity;
match theme_config.popup_backdrop.as_str() {
    "none" => css.load_from_string("window.backdrop { background: transparent; }"),
    "dim" => css.load_from_string(&format!("window.backdrop {{ background: rgba(0,0,0,{opacity}); }}")),
    _ => css.load_from_string("window.backdrop { background: rgba(0,0,0,0.95); }"),
};
backdrop_win.add_css_class("backdrop");
gtk4::style_context_add_provider_for_display(
    &gtk4::gdk::Display::default().unwrap(),
    &css,
    gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
);
```

For `"none"` mode, also skip showing the backdrop in the render loop:
```rust
// In the 80ms timer:
if state.popup.is_empty() {
    if theme_config.popup_backdrop != "none" {
        backdrop_win.set_visible(false);
    }
    popup_win.set_visible(false);
} else {
    if theme_config.popup_backdrop != "none" {
        backdrop_win.set_visible(true);
    }
    popup_win.set_visible(true);
    // ...
}
```

- [ ] **Step 3: Update Zenith config.json**

```json
{
  "bar_height": 36,
  "bar_position": "top",
  "popup_width": 560,
  "popup_height": 560,
  "popup_backdrop": "dim",
  "popup_dim_opacity": 0.25
}
```

- [ ] **Step 4: Build and test**

Run: `cargo build -p pulpkit-webshell-poc`
Test: run shell, open a popup, verify desktop is visible with dim overlay behind it.

---

## Task 2: Lock Screen IPC Command (Rust)

**Files:**
- Modify: `poc/src/main.rs`

- [ ] **Step 1: Add verify_password command to handle_command**

```rust
"verify_password" => {
    if let Some(password) = data.as_str() {
        let ok = crate::pam::verify_password(password);
        // Store result in custom state so JS can read it
        s.custom.insert("auth_result".to_string(),
            serde_json::Value::Bool(ok));
        s.dirty.store(true, Ordering::Relaxed);
    }
}
```

- [ ] **Step 2: Build and test**

Run: `cargo build -p pulpkit-webshell-poc`

---

## Task 3: Bar Indicators Component

**Files:**
- Create: `poc/shells/zenith/components/indicators.js`
- Modify: `poc/shells/zenith/bar.html` — add indicator container + script import

- [ ] **Step 1: Create indicators.js**

Contextual indicators that appear/disappear based on state. Each indicator is a small icon in the bar's right section, between existing status items.

```javascript
function renderIndicators(s) {
  const el = document.getElementById('indicators');
  if (!el) return;

  let html = '';

  // VPN
  if (s.vpn_active)
    html += `<span class="ind" title="VPN: ${s.vpn_name||'Connected'}">󰦝</span>`;

  // DND
  if (s.dnd)
    html += '<span class="ind dim" title="Do Not Disturb">󰍶</span>';

  // Caffeine
  if (s.caffeine_active)
    html += '<span class="ind" title="Caffeine active">󰛊</span>';

  // Gaming
  if (s.gamemode_active)
    html += `<span class="ind accent" title="GameMode (GPU ${s.gpu_temp||0}°C)">󰊗 ${s.gpu_temp?s.gpu_temp+'°':''}</span>`;

  // Mic (when unmuted and in use)
  if (s.mic_volume > 0 && !s.mic_muted)
    html += '<span class="ind" title="Mic active">󰍬</span>';
  else if (s.mic_muted)
    html += '<span class="ind dim" title="Mic muted">󰍭</span>';

  // Screen sharing
  if (s.screen_sharing)
    html += '<span class="ind warn" title="Screen sharing">󰍹</span>';

  el.innerHTML = html;
}
```

- [ ] **Step 2: Add indicator container to bar.html**

In the right section of bar.html, add between existing elements:
```html
<div id="indicators" style="display:flex;gap:6px;align-items:center;font-size:13px;"></div>
```

Add script import at the bottom:
```html
<script src="components/indicators.js"></script>
```

Call `renderIndicators(s)` from the main `updateState()` function.

- [ ] **Step 3: Test**

Run shell, verify indicators appear when relevant state is set (use `set_mock_state` via MCP to test).

---

## Task 4: Enhance Settings Panel

**Files:**
- Modify: `poc/shells/zenith/popup.html` — settings panel section

- [ ] **Step 1: Add new quick tiles**

Add to the existing tile grid in `#panel-settings`:
- Caffeine tile (toggles `$XDG_RUNTIME_DIR/pulpkit-caffeine` marker via exec)
- VPN tile (shows vpn_active state, click → exec toggle if available)
- Gamemode tile (shows gamemode_active, informational)

- [ ] **Step 2: Add mic volume slider**

Below the existing volume and brightness sliders, add a mic volume slider:
```html
<div class="slider-row">
  <span class="slider-icon" id="mic-icon">󰍬</span>
  <input type="range" id="mic-slider" min="0" max="100">
  <span class="slider-val" id="mic-val">0%</span>
</div>
```

Wire up: read `s.mic_volume`, `s.mic_muted`. On change, exec `wpctl set-volume @DEFAULT_AUDIO_SOURCE@ {val}%`.

- [ ] **Step 3: Enhance battery row**

Show power draw: `s.power_draw_watts` formatted as "12.3W"
Show AC status: plug icon when `s.ac_plugged`

- [ ] **Step 4: Test**

Use MCP `set_mock_state` to verify new tiles and sliders render correctly.

---

## Task 5: Enhance System Monitor Panel

**Files:**
- Modify: `poc/shells/zenith/popup.html` — monitor panel section

- [ ] **Step 1: Add GPU section**

When `s.gpu_usage > 0` or `s.gpu_temp > 0`, show:
```
GPU  [========  ] 45%  65°C
VRAM [====      ] 2048/8192 MB
```

- [ ] **Step 2: Add network speed**

Format `s.net_rx_bytes_sec` and `s.net_tx_bytes_sec` as human-readable (KB/s, MB/s):
```
Network  ↓ 1.2 MB/s  ↑ 45 KB/s
```

- [ ] **Step 3: Add containers section**

When `s.containers.length > 0`, show container list with name, image, status.

- [ ] **Step 4: Add failed units warning**

When `s.failed_unit_count > 0`, show red warning with list of `s.failed_units`.

- [ ] **Step 5: Add fan speed, load average, swap**

```
Fan: 1200 RPM | Load: 2.1 1.8 1.5 | Swap: 512/8192 MB
```

- [ ] **Step 6: Test**

Verify all new sections render with mock state.

---

## Task 6: Enhance Notifications Panel

**Files:**
- Modify: `poc/shells/zenith/popup.html` — notifications panel section

- [ ] **Step 1: Wire notifications from daemon**

The panel already exists and renders `s.notifications`. Enhance:
- Add DND toggle at top
- Show notification body (expandable on click)
- Add "Clear All" button that sends `{cmd: "exec", data: "busctl --user call org.freedesktop.Notifications /org/freedesktop/Notifications org.freedesktop.Notifications CloseNotification u <id>"}`
- Actually, add a new command: `notif_clear_all` that clears all notifications from state

- [ ] **Step 2: Add notif_clear_all command to Rust**

In `handle_command()`:
```rust
"notif_clear_all" => {
    // Clear notifications from polled state
    if let Ok(mut ps) = polled.lock() {
        ps.notifications.clear();
        ps.notif_count = 0;
    }
    s.dirty.store(true, Ordering::Relaxed);
}
```

Wait — `handle_command` doesn't have access to `polled_state`. The notifications are managed by the daemon thread. We need a different approach.

Better: send a command via the IPC socket to clear, or add a channel. Simplest: just dismiss each notification via the daemon's CloseNotification method. The JS can call `send({cmd: 'exec', data: 'for id in $(seq 1 100); do busctl ...; done'})`.

Actually simplest: the notification daemon already manages `FullState.notifications`. Add a new command that the JS sends, and `handle_command` clears the vec:

Since `handle_command` gets `&Rc<RefCell<AppState>>` (not `FullState`), we need the polled state. The cleanest approach: add `notif_clear_all` as a flag in AppState that the 80ms timer checks.

Simplest: just exec `notify-send` replacement. Actually, the notifications vec is in `polled_state` which the 80ms timer reads. We can add a "clear notifications" flag to AppState:

```rust
// In AppState:
pub clear_notifications: bool,

// In handle_command:
"notif_clear_all" => {
    s.clear_notifications = true;
    s.dirty.store(true, Ordering::Relaxed);
}

// In 80ms timer, before merging state:
if app_state.borrow().clear_notifications {
    if let Ok(mut ps) = polled.lock() {
        ps.notifications.clear();
        ps.notif_count = 0;
    }
    app_state.borrow_mut().clear_notifications = false;
}
```

- [ ] **Step 3: Test**

Send `notify-send "Test" "Hello"`, verify it appears in panel. Click clear, verify it disappears.

---

## Task 7: Audio Mixer Panel

**Files:**
- Modify: `poc/shells/zenith/popup.html` — add audio panel

- [ ] **Step 1: Add audio mixer panel HTML**

```html
<div id="panel-audio" class="panel">
  <div class="panel-title">
    <span class="panel-title-icon">󰕾</span><span>Audio</span>
  </div>
  <div id="audio-content"></div>
</div>
```

- [ ] **Step 2: Write renderAudio function**

```javascript
function renderAudio(s) {
  const el = document.getElementById('audio-content');
  if (!el || s.popup !== 'audio') return;

  let html = '<div class="section-label">Output Devices</div>';
  (s.audio_sinks || []).forEach(d => {
    const active = d.active ? ' active' : '';
    html += `<div class="audio-device${active}" onclick="send({cmd:'exec',data:'wpctl set-default ${d.name}'})">
      <span>󰓃</span><span>${d.description}</span>
      ${d.active ? '<span class="accent">✓</span>' : ''}
    </div>`;
  });

  html += '<div class="section-label" style="margin-top:12px">Input Devices</div>';
  (s.audio_sources || []).forEach(d => {
    const active = d.active ? ' active' : '';
    html += `<div class="audio-device${active}" onclick="send({cmd:'exec',data:'wpctl set-default ${d.name}'})">
      <span>󰍬</span><span>${d.description}</span>
      ${d.active ? '<span class="accent">✓</span>' : ''}
    </div>`;
  });

  if ((s.audio_streams || []).length > 0) {
    html += '<div class="section-label" style="margin-top:12px">App Volume</div>';
    s.audio_streams.forEach(st => {
      const icon = st.is_input ? '󰍬' : '󰕾';
      html += `<div class="stream-row">
        <span>${icon}</span>
        <span class="stream-name">${st.app_name || st.name}</span>
        <input type="range" min="0" max="150" value="${st.volume}"
          oninput="send({cmd:'exec',data:'pactl set-sink-input-volume ... '})">
        <span>${st.volume}%</span>
      </div>`;
    });
  }

  el.innerHTML = html;
}
```

- [ ] **Step 3: Add popup toggle for audio panel**

Add button in bar or settings that opens `popup: 'audio'`.

- [ ] **Step 4: Test**

Open audio panel, verify devices and streams render. Test device switching.

---

## Task 8: Enhance Calendar/Weather Panel

**Files:**
- Modify: `poc/shells/zenith/popup.html` — calendar panel section

- [ ] **Step 1: Add weather row**

Above the calendar grid, add weather display:
```javascript
// In renderCalendar:
let weatherHtml = '';
if (s.weather_temp || s.weather_condition) {
  weatherHtml = `<div class="weather-row">
    <span>${s.weather_icon || '🌤'}</span>
    <span>${s.weather_temp ? Math.round(s.weather_temp) + '°' : ''}</span>
    <span class="dim">${s.weather_condition || ''}</span>
    <span class="flex"></span>
    <span class="dim">☀ ${s.sunrise || ''} 🌙 ${s.sunset || ''}</span>
  </div>`;
}
```

- [ ] **Step 2: Add upcoming events**

Below the calendar grid:
```javascript
if ((s.calendar_events || []).length > 0) {
  html += '<div class="section-label">Upcoming</div>';
  s.calendar_events.slice(0, 5).forEach(ev => {
    html += `<div class="event-row">
      <span class="accent">●</span>
      <span>${ev.summary}</span>
      <span class="dim">${ev.start}</span>
    </div>`;
  });
}
```

- [ ] **Step 3: Test**

Use mock state with weather and calendar data.

---

## Task 9: Enhance Config/Settings App

**Files:**
- Modify: `poc/shells/zenith/popup.html` — config panel section

The settings app already has 9 pages. Enhance with new state fields:

- [ ] **Step 1: Network page — add VPN status, network speed**

```javascript
// In network page render:
if (s.vpn_active) {
  addRow('VPN', s.vpn_name, '󰦝');
}
addRow('Download', formatBytes(s.net_rx_bytes_sec) + '/s', '󰇚');
addRow('Upload', formatBytes(s.net_tx_bytes_sec) + '/s', '󰕒');
```

- [ ] **Step 2: Power page — add power draw, AC status**

```javascript
addRow('Power Draw', s.power_draw_watts ? s.power_draw_watts.toFixed(1) + 'W' : '—', '󰚥');
addRow('AC Power', s.ac_plugged ? 'Plugged in' : 'On battery', s.ac_plugged ? '󰚥' : '󰚦');
```

- [ ] **Step 3: About page — add more system info**

Add: compositor, timezone, keyboard layout, load averages, swap, fan RPM, failed units count.

- [ ] **Step 4: Add new pages: Gaming, Containers**

Gaming page (if gamemode or GPU data available):
- GameMode status
- GPU usage/temp/VRAM
- Gamescope status
- Discord activity

Containers page (if containers present):
- List of running containers with name, image, status

- [ ] **Step 5: Test**

Navigate through all settings pages, verify new data renders.

---

## Task 10: Lock Screen

**Files:**
- Create: `poc/shells/zenith/lock.html`
- Modify: `poc/src/main.rs` — lock window creation + management

- [ ] **Step 1: Create lock.html**

Full-screen lock surface with:
- Large centered clock
- Date
- User avatar (if `s.user_icon` is set) or user initial
- Password input field
- Error message area
- Subtle background (solid dark with gradient or blurred)

```html
<!DOCTYPE html>
<html>
<head>
<style>
  * { margin: 0; padding: 0; box-sizing: border-box; }
  body {
    background: #181825;
    color: #cdd6f4;
    font-family: 'Inter', sans-serif;
    height: 100vh;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    user-select: none;
  }
  .clock { font-size: 72px; font-weight: 200; letter-spacing: -2px; }
  .date { font-size: 16px; opacity: 0.6; margin-top: 4px; }
  .avatar {
    width: 80px; height: 80px; border-radius: 50%;
    background: #313244; margin: 40px 0 16px;
    display: flex; align-items: center; justify-content: center;
    font-size: 32px; overflow: hidden;
  }
  .avatar img { width: 100%; height: 100%; object-fit: cover; }
  .username { font-size: 14px; opacity: 0.6; margin-bottom: 20px; }
  .input-wrap {
    background: #1e1e2e; border: 1px solid #313244;
    border-radius: 24px; padding: 10px 20px; width: 280px;
    display: flex; align-items: center; gap: 8px;
  }
  .input-wrap.error { border-color: #f38ba8; animation: shake 0.3s; }
  .input-wrap input {
    background: transparent; border: none; outline: none;
    color: #cdd6f4; font-size: 14px; flex: 1;
  }
  .input-wrap .lock-icon { opacity: 0.4; }
  .error-msg { color: #f38ba8; font-size: 12px; margin-top: 8px; height: 16px; }
  @keyframes shake {
    0%,100% { transform: translateX(0); }
    25% { transform: translateX(-8px); }
    75% { transform: translateX(8px); }
  }
</style>
</head>
<body>
  <div class="clock" id="lock-clock"></div>
  <div class="date" id="lock-date"></div>
  <div class="avatar" id="lock-avatar"></div>
  <div class="username" id="lock-user"></div>
  <div class="input-wrap" id="pw-wrap">
    <span class="lock-icon">󰌾</span>
    <input type="password" id="pw-input" placeholder="Password" autofocus>
  </div>
  <div class="error-msg" id="pw-error"></div>
  <script>
    function send(o) {
      window.webkit.messageHandlers.pulpkit.postMessage(JSON.stringify(o));
    }

    // Clock
    function updateClock() {
      const now = new Date();
      document.getElementById('lock-clock').textContent =
        now.toLocaleTimeString([], {hour:'2-digit', minute:'2-digit', hour12: false});
      document.getElementById('lock-date').textContent =
        now.toLocaleDateString([], {weekday:'long', month:'long', day:'numeric'});
    }
    updateClock();
    setInterval(updateClock, 1000);

    // Password input
    const input = document.getElementById('pw-input');
    const wrap = document.getElementById('pw-wrap');
    const errEl = document.getElementById('pw-error');

    input.addEventListener('keydown', e => {
      if (e.key === 'Enter' && input.value) {
        send({cmd: 'verify_password', data: input.value});
        input.value = '';
      }
      wrap.classList.remove('error');
      errEl.textContent = '';
    });

    function updateState(s) {
      // Set user info
      const avatar = document.getElementById('lock-avatar');
      if (s.user_icon) {
        avatar.innerHTML = `<img src="file://${s.user_icon}">`;
      } else {
        avatar.textContent = (s.user || '?')[0].toUpperCase();
      }
      document.getElementById('lock-user').textContent = s.user || '';

      // Check auth result
      if (s.custom && s.custom.auth_result === true) {
        send({cmd: 'unlock'});
      } else if (s.custom && s.custom.auth_result === false) {
        wrap.classList.add('error');
        errEl.textContent = 'Incorrect password';
        s.custom.auth_result = null;
      }
    }
  </script>
</body>
</html>
```

- [ ] **Step 2: Add lock window to Rust**

This is the most complex Rust change. When `session_locked` becomes true in the logind watcher:
- Create a new `Layer::Overlay` window anchored to all edges (full screen)
- Load `lock.html` into it
- Set keyboard mode to `Exclusive` (captures all keyboard input)
- Hide bar and popup

When auth succeeds (`unlock` command):
- Destroy lock window
- Show bar again
- Call `loginctl unlock-session`

This requires tracking the lock window as an `Option<gtk4::Window>` in the main closure. Since this is complex GTK lifecycle work, implement as a new command handler:

```rust
"unlock" => {
    spawn_quiet("loginctl", &["unlock-session"]);
    // Lock window cleanup handled by session_locked becoming false
}
```

The lock window creation is triggered by the 80ms timer detecting `state.session_locked` transition from false → true.

- [ ] **Step 3: Test**

Run: `loginctl lock-session`
Verify: lock screen appears with clock, avatar, password input.
Type password, press Enter, verify unlock.

---

## Implementation Order

1. Task 1: Configurable backdrop (Rust) — foundation for the visual change
2. Task 2: Lock screen IPC (Rust) — verify_password command
3. Task 3: Bar indicators (JS) — quick visual win
4. Task 4: Settings panel enhancements (JS)
5. Task 5: System monitor enhancements (JS)
6. Task 6: Notifications panel (JS + small Rust change)
7. Task 7: Audio mixer panel (JS)
8. Task 8: Calendar/weather (JS)
9. Task 9: Config/settings app pages (JS)
10. Task 10: Lock screen (HTML + Rust)

Tasks 3-9 are all independent JS work and can be parallelized.
