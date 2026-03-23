# Your First Shell

This tutorial walks through creating a shell from scratch. By the end you will have a working bar with workspaces, a clock, volume control, and a popup panel.

## Create the Shell Directory

Every shell lives under `poc/shells/`. Create yours:

```bash
mkdir -p poc/shells/myshell
```

You need at minimum three files:

```
poc/shells/myshell/
├── bar.html       # Status bar (required)
├── popup.html     # Popup panels (required)
└── config.json    # Dimensions and positioning
```

## config.json

Start with a basic configuration:

```json
{
  "bar_height": 36,
  "bar_position": "top",
  "popup_width": 360,
  "popup_height": 480,
  "popup_backdrop": "dim",
  "popup_dim_opacity": 0.25
}
```

| Field | Description |
|-------|-------------|
| `bar_height` | Bar thickness in pixels |
| `bar_position` | `"top"` or `"bottom"` |
| `popup_width` | Popup surface width |
| `popup_height` | Popup surface height |
| `popup_backdrop` | `"none"`, `"dim"`, or `"opaque"` |
| `popup_dim_opacity` | Backdrop opacity when `"dim"` (0.0 - 1.0) |

## Minimal bar.html

Every HTML file must define two things:

1. **`updateState(s)`** — called by Pulpkit with the full state object
2. **`send(o)`** — sends commands back to the Rust backend

Here is a bare-bones bar:

```html
<!DOCTYPE html>
<html>
<head>
<style>
  * { margin: 0; padding: 0; box-sizing: border-box; }
  body {
    font-family: 'Inter', sans-serif;
    font-size: 13px;
    background: transparent;
    color: #cdd6f4;
    height: 100vh;
    display: flex;
    align-items: center;
    padding: 0 12px;
    overflow: hidden;
    user-select: none;
  }
  .bar {
    display: flex;
    align-items: center;
    width: 100%;
    background: #1e1e2e;
    height: 32px;
    padding: 0 12px;
    border-radius: 0;
  }
  .left, .center, .right {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .left { flex: 1; }
  .center { flex: 0; }
  .right { flex: 1; justify-content: flex-end; }
</style>
</head>
<body>
<div class="bar">
  <div class="left" id="left"></div>
  <div class="center" id="center"></div>
  <div class="right" id="right"></div>
</div>
<script>
function send(o) {
  window.webkit.messageHandlers.pulpkit.postMessage(JSON.stringify(o));
}

function updateState(s) {
  // We will fill this in step by step
}
</script>
</body>
</html>
```

!!! note "Transparent background"
    The `body` background must be `transparent`. The layer-shell surface is transparent by default — your HTML content floats directly over the desktop.

## Minimal popup.html

The popup receives the same state. The `s.popup` field tells you which panel to show:

```html
<!DOCTYPE html>
<html>
<head>
<style>
  * { margin: 0; padding: 0; box-sizing: border-box; }
  body {
    font-family: 'Inter', sans-serif;
    font-size: 13px;
    background: transparent;
    color: #cdd6f4;
    overflow: hidden;
    user-select: none;
  }
  .panel {
    display: none;
    flex-direction: column;
    gap: 12px;
    background: #1e1e2e;
    padding: 16px;
    width: 100%;
    height: 100vh;
  }
  .panel.active { display: flex; }
</style>
</head>
<body>

<div class="panel" id="panel-settings">
  <h3 style="font-size: 14px; color: #a6adc8;">Settings</h3>
  <p id="settings-info"></p>
</div>

<script>
function send(o) {
  window.webkit.messageHandlers.pulpkit.postMessage(JSON.stringify(o));
}

function updateState(s) {
  document.querySelectorAll('.panel').forEach(p => p.classList.remove('active'));
  const panel = document.getElementById('panel-' + s.popup);
  if (panel) panel.classList.add('active');

  document.getElementById('settings-info').textContent =
    'Volume: ' + s.vol + '% | Brightness: ' + s.bright + '%';
}
</script>
</body>
</html>
```

## Run It

```bash
./target/release/pulpkit-webshell-poc myshell
```

You should see an empty bar. Now let's add widgets.

## Add Workspace Dots

Workspaces arrive in `s.ws` as an array of `{ idx, active }` objects. Render them as clickable dots:

```javascript
function updateState(s) {
  // Workspaces
  const left = document.getElementById('left');
  left.innerHTML = '';
  s.ws.forEach(w => {
    const dot = document.createElement('span');
    dot.style.cssText = `
      width: 8px; height: 8px; border-radius: 50%;
      background: ${w.active ? '#89b4fa' : '#45475a'};
      cursor: pointer; transition: background 0.2s;
    `;
    dot.onclick = () => send({ cmd: 'ws_go', data: w.idx });
    left.appendChild(dot);
  });
}
```

Each dot is clickable — `send({ cmd: 'ws_go', data: w.idx })` switches to that workspace.

## Add a Clock

The clock is pure JavaScript — no state field needed:

```javascript
function updateClock() {
  const d = new Date();
  const h = String(d.getHours()).padStart(2, '0');
  const m = String(d.getMinutes()).padStart(2, '0');
  document.getElementById('center').textContent = h + ':' + m;
}
updateClock();
setInterval(updateClock, 1000);
```

!!! tip "Clocks don't need state"
    The browser has its own clock. No need to waste a state field on it. Use `setInterval` for time display and reserve `updateState` for system data.

## Add Volume Control

Display the volume level and toggle mute on right-click:

Add this CSS inside your `<style>` block:

```css
.stat {
  font-size: 12px;
  padding: 4px 8px;
  cursor: pointer;
  color: #a6adc8;
  transition: color 0.2s;
}
.stat:hover { color: #cdd6f4; }
.stat.muted { color: #f38ba8; }
```

Add a span in the bar's right section (in the HTML body):

```html
<div class="right" id="right">
  <span class="stat" id="volume"
    onclick="send({cmd:'popup',data:'settings'})"
    oncontextmenu="event.preventDefault(); send({cmd:'vol_mute'})">
  </span>
</div>
```

Update it in your `updateState` function:

```javascript
// Inside updateState(s):
const vol = document.getElementById('volume');
vol.textContent = s.muted ? 'muted' : 'vol ' + s.vol + '%';
vol.className = 'stat' + (s.muted ? ' muted' : '');
```

## Putting It All Together

Here is the complete `bar.html` with all three widgets:

```html
<!DOCTYPE html>
<html>
<head>
<style>
  * { margin: 0; padding: 0; box-sizing: border-box; }
  body {
    font-family: 'Inter', sans-serif;
    font-size: 13px;
    background: transparent;
    color: #cdd6f4;
    height: 100vh;
    display: flex;
    align-items: center;
    overflow: hidden;
    user-select: none;
  }
  .bar {
    display: flex;
    align-items: center;
    width: 100%;
    background: #1e1e2e;
    height: 32px;
    padding: 0 16px;
  }
  .left, .center, .right {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .left { flex: 1; }
  .center { flex: 0; white-space: nowrap; font-weight: 600; }
  .right { flex: 1; justify-content: flex-end; }
  .stat {
    font-size: 12px;
    padding: 4px 8px;
    cursor: pointer;
    color: #a6adc8;
    transition: color 0.2s;
  }
  .stat:hover { color: #cdd6f4; }
  .stat.muted { color: #f38ba8; }
</style>
</head>
<body>
<div class="bar">
  <div class="left" id="left"></div>
  <div class="center" id="center"></div>
  <div class="right" id="right">
    <span class="stat" id="volume"
      onclick="send({cmd:'popup',data:'settings'})"
      oncontextmenu="event.preventDefault(); send({cmd:'vol_mute'})">
    </span>
  </div>
</div>
<script>
function send(o) {
  window.webkit.messageHandlers.pulpkit.postMessage(JSON.stringify(o));
}

// Clock — runs independently of state
function updateClock() {
  const d = new Date();
  const h = String(d.getHours()).padStart(2, '0');
  const m = String(d.getMinutes()).padStart(2, '0');
  document.getElementById('center').textContent = h + ':' + m;
}
updateClock();
setInterval(updateClock, 1000);

function updateState(s) {
  // Workspace dots
  const left = document.getElementById('left');
  left.innerHTML = '';
  s.ws.forEach(w => {
    const dot = document.createElement('span');
    dot.style.cssText = `
      width: 8px; height: 8px; border-radius: 50%;
      background: ${w.active ? '#89b4fa' : '#45475a'};
      cursor: pointer; transition: background 0.2s;
    `;
    dot.onclick = () => send({ cmd: 'ws_go', data: w.idx });
    left.appendChild(dot);
  });

  // Volume
  const vol = document.getElementById('volume');
  vol.textContent = s.muted ? 'muted' : 'vol ' + s.vol + '%';
  vol.className = 'stat' + (s.muted ? ' muted' : '');
}
</script>
</body>
</html>
```

## What's Next

You now have a working shell with workspaces, a clock, and volume control. From here you can:

- **Add more state fields** — `s.wifi`, `s.bat`, `s.cpu`, `s.mem`, `s.media_title`, and 180+ more
- **Build popup panels** — Settings, WiFi picker, app launcher, power menu
- **Add a toast surface** — `toast.html` for notification popups
- **Use themes** — Apply CSS variables with `s.theme` and `send({ cmd: 'set_theme', data: 'nord' })`
- **Add a lock screen** — `lock.html` with PAM authentication

!!! example "Explore built-in shells"
    Look at `poc/shells/minimal/` for a clean reference, or `poc/shells/zenith/` for a fully-featured example with panels, tray icons, and media controls.
