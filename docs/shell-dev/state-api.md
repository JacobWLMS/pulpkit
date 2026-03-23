# State API

Pulpkit pushes system state to shell webviews as a JSON object. Your shell receives
this state through the `updateState(s)` function, which is called approximately every
80ms whenever any monitored value changes.

## The `updateState(s)` Contract

Every shell HTML file (bar.html, popup.html, toast.html) must define a global function:

```js
function updateState(s) {
  // s is the full system state object
  // Render your UI from s
}
```

The runtime calls this function by evaluating:

```js
if(typeof updateState==='function') updateState({...json...})
```

!!! info "Call frequency"
    `updateState()` is called at most once every 80ms. The runtime deduplicates
    identical state -- if nothing changed since the last push, the function is
    not called.

## How State Is Pushed

```
Rust watchers ──→ FullState struct ──→ serde_json::to_string ──→ WebView.evaluate_javascript
                  (Arc<Mutex>)         (JSON string)              ("updateState({json})")
```

1. **Watchers** monitor system resources via DBus, procfs, inotify, and Wayland protocols
2. Each watcher updates fields in the shared `FullState` struct and sets a dirty flag
3. Every 80ms, the main thread checks the dirty flag
4. If dirty, the full state is serialized to JSON and compared with the last push
5. If different, `updateState(s)` is called in all active webviews (bar, popup, toast)

## State Object Shape

The state object `s` contains every field from the [state reference](../reference/state-fields.md).
Here is a representative example:

```json
{
  "vol": 72,
  "muted": false,
  "audio_device": "HD Audio Speaker",
  "bright": 65,
  "bat": 78,
  "bat_status": "Discharging",
  "has_bat": true,
  "cpu": 23,
  "mem": 45,
  "disk_used": "171G",
  "disk_total": "248G",
  "disk_pct": 69,
  "power_profile": "balanced",
  "wifi": "HomeNetwork",
  "net_signal": 82,
  "net_ip": "192.168.1.42",
  "notif_count": 2,
  "dnd": false,
  "ws": [
    {"idx": 1, "active": true},
    {"idx": 2, "active": false},
    {"idx": 3, "active": false}
  ],
  "windows": [
    {"id": 1, "title": "Firefox", "app_id": "firefox", "focused": true, "icon": ""},
    {"id": 2, "title": "Terminal", "app_id": "kitty", "focused": false, "icon": ""}
  ],
  "active_title": "Firefox",
  "active_app_id": "firefox",
  "popup": "settings",
  "theme": "gruvbox",
  "custom": {},
  "user": "jacob",
  "host": "archlinux",
  "kernel": "6.19.7-1-cachyos",
  "uptime": "3 hours, 12 min",
  "media_playing": false,
  "media_title": "",
  "media_artist": "",
  "bt_powered": true,
  "bt_connected": [],
  "notifications": [],
  "session_locked": false,
  "gamemode_active": false,
  "night_light_active": false
}
```

!!! note "All fields are always present"
    The state object is serialized from a Rust struct with `#[derive(Serialize, Default)]`.
    Every field is always present in the JSON -- strings default to `""`, numbers to `0`,
    booleans to `false`, and arrays to `[]`.

## Defensive Coding

Although all fields are guaranteed present, defensive checks are good practice when
accessing nested data or `custom` keys:

```js
function updateState(s) {
  // Safe: top-level fields are always present
  const vol = s.vol;  // always a number

  // Defensive: custom keys may or may not exist
  const authResult = s.custom && s.custom.auth_result;

  // Defensive: array items may have unexpected shapes during transitions
  if (s.ws && s.ws.length > 0) {
    renderWorkspaces(s.ws);
  }

  // Defensive: string fields may be empty
  const wifiLabel = s.wifi || "Disconnected";
}
```

## Memoization Pattern

Since `updateState()` is called frequently (~12 times/second), avoid unnecessary DOM
updates. The recommended pattern compares values before touching the DOM:

### `setText()` — Update text only if changed

```js
function setText(id, text) {
  const el = document.getElementById(id);
  if (el && el.textContent !== text) el.textContent = text;
}

// Usage
function updateState(s) {
  setText('vol-label', s.vol + '%');
  setText('wifi-label', s.wifi || 'Off');
}
```

### `setHtml()` — Update innerHTML only if changed

```js
function setHtml(id, html) {
  const el = document.getElementById(id);
  if (el && el.innerHTML !== html) el.innerHTML = html;
}
```

### `updateList()` — Smart list reconciliation

For dynamic lists (workspaces, windows, wifi networks), avoid rebuilding the DOM
on every call. The `updateList()` helper uses a key-based check:

```js
function updateList(containerId, items, keyFn, renderFn) {
  const el = document.getElementById(containerId);
  if (!el) return;
  const newKey = items.map(keyFn).join(',');
  if (el.dataset.key === newKey) return; // no change
  el.dataset.key = newKey;
  const scroll = el.scrollTop;
  el.innerHTML = '';
  items.forEach(item => el.appendChild(renderFn(item)));
  el.scrollTop = scroll;  // preserve scroll position
}

// Usage
function renderWorkspaces(s) {
  updateList('ws-container', s.ws,
    w => `${w.idx}:${w.active}`,
    w => {
      const btn = document.createElement('button');
      btn.textContent = w.idx;
      btn.className = w.active ? 'ws active' : 'ws';
      btn.onclick = () => send({cmd: 'ws_go', data: w.idx});
      return btn;
    }
  );
}
```

### `sendThrottled()` — Rate-limit rapid commands

For sliders (volume, brightness), throttle to at most one command per 80ms:

```js
function sendThrottled(key, o) {
  if (window['_t_' + key]) return;
  send(o);
  window['_t_' + key] = setTimeout(() => { delete window['_t_' + key]; }, 80);
}

// Usage
slider.oninput = (e) => {
  sendThrottled('vol', {cmd: 'vol_set', data: parseInt(e.target.value)});
};
```

## Global State Reference

Store the latest state globally so event handlers can access it:

```js
let _st = {};

function updateState(s) {
  _st = s;
  // ... render
}

// Later, in an onclick handler:
button.onclick = () => {
  send({cmd: 'vol_mute'});
  // Can reference _st.vol, _st.muted, etc.
};
```

## Custom State

The `s.custom` object is a key-value store you can use for shell-specific state.
Set values with the `set_custom` command:

```js
send({cmd: 'set_custom', data: {key: 'my_toggle', value: true}});
```

On the next `updateState()` call, `s.custom.my_toggle` will be `true`.

This is useful for:

- Lock screen authentication flow (`s.custom.auth_result`)
- Shell-specific settings (accent colors, layout toggles)
- Inter-surface communication (bar sets a value, popup reads it)
