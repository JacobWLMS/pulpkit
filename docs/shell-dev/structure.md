# Shell File Structure

Pulpkit shells are directories containing HTML, CSS, and JavaScript files that define
the bar, popup, toast, and lock screen surfaces. The runtime loads these files at
startup and renders them in WebKitGTK webviews on Wayland layer-shell surfaces.

## Shell Directory Layout

```
poc/shells/my-shell/
  bar.html              # Required — status bar
  popup.html            # Required — popup panels
  config.json           # Required — dimensions and behavior
  theme.css             # Optional — color theme variables
  toast.html            # Optional — notification toasts
  lock.html             # Optional — lock screen
  components/           # Optional — bar component scripts
    workspaces.js
    taskbar.js
    clock.js
    status.js
    tray.js
  panels/               # Optional — popup panel scripts
    settings.js
    wifi.js
    power.js
    launcher.js
    config.js
```

## Required Files

### `bar.html`

The status bar. Rendered in a layer-shell surface anchored to the top or bottom edge
of the screen.

**Contract:**

- Must define `function updateState(s)` — called every ~80ms when state changes
- Must define `function send(o)` — sends commands to the Rust backend

```html
<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <link rel="stylesheet" href="theme.css">
</head>
<body>
  <div id="bar">...</div>
  <script>
    function send(o) {
      window.webkit.messageHandlers.pulpkit.postMessage(JSON.stringify(o));
    }
    function updateState(s) {
      if (s.theme) document.documentElement.setAttribute('data-theme', s.theme);
      // render bar content from s
    }
  </script>
</body>
</html>
```

### `popup.html`

The popup overlay. Shown when the user opens a panel (settings, wifi, power, etc.).
Rendered in a centered layer-shell surface with optional backdrop dimming.

**Contract:**

- Same `updateState(s)` and `send(o)` contract as bar.html
- Panels use `id="panel-{name}"` with the CSS class `active` to show/hide
- Must toggle panel visibility based on `s.popup`

```html
<div id="panel-settings" class="panel"></div>
<div id="panel-wifi" class="panel"></div>
<div id="panel-power" class="panel"></div>
<div id="panel-launcher" class="panel"></div>

<script>
function updateState(s) {
  if (s.theme) document.documentElement.setAttribute('data-theme', s.theme);
  // Toggle active panel
  document.querySelectorAll('.panel').forEach(p => p.classList.remove('active'));
  const panel = document.getElementById('panel-' + s.popup);
  if (panel) panel.classList.add('active');
  // render panel contents
}
</script>
```

### `config.json`

Shell dimensions and behavior. All fields have defaults; you only need to specify
values that differ from defaults.

```json
{
  "bar_height": 40,
  "bar_position": "top",
  "popup_width": 380,
  "popup_height": 500,
  "popup_backdrop": "dim",
  "popup_dim_opacity": 0.25
}
```

See the [config.json reference](../reference/config.md) for full details.

## Optional Files

### `theme.css`

CSS custom property definitions for color themes. If present, link it from bar.html
and popup.html with `<link rel="stylesheet" href="theme.css">`. If omitted, the
runtime injects the built-in theme variables automatically.

### `toast.html`

Notification toast surface. Rendered in a layer-shell surface anchored to the top-right
corner. Receives the same `updateState(s)` calls as bar.html and popup.html. If omitted,
a blank fallback is used.

See the [toasts guide](toasts.md) for full details.

### `lock.html`

Lock screen surface. Rendered when the session is locked via `ext-session-lock-v1`.
If omitted, the session lock protocol is not used.

See the [lock screen guide](lock-screen.md) for full details.

## Component Organization Pattern

For maintainability, the `scaffold_shell` MCP tool creates a component-based structure
where bar.html and popup.html are thin skeletons that load component scripts.

### Bar Components

Each component exports a `render{Name}(s)` function called from `updateState()`:

```
bar.html
  └── <script src="components/workspaces.js">   → renderWorkspaces(s)
  └── <script src="components/taskbar.js">       → renderTaskbar(s)
  └── <script src="components/clock.js">         → renderClock(s)
  └── <script src="components/status.js">        → renderStatus(s)
  └── <script src="components/tray.js">          → renderTray(s)
```

bar.html orchestrates them:

```js
function updateState(s) {
  _st = s;
  if (s.theme) document.documentElement.setAttribute('data-theme', s.theme);
  renderWorkspaces(s);
  renderTaskbar(s);
  renderClock(s);
  renderStatus(s);
  renderTray(s);
}
```

### Popup Panels

Each panel exports a `render{Name}(s)` function:

```
popup.html
  └── <script src="panels/settings.js">    → renderSettings(s)
  └── <script src="panels/wifi.js">        → renderWifi(s)
  └── <script src="panels/power.js">       → renderPower(s)
  └── <script src="panels/launcher.js">    → renderLauncher(s)
  └── <script src="panels/config.js">      → renderConfig(s)
```

!!! tip "Custom components"
    You are not limited to the default component names. Add any `.js` files you want
    and call them from `updateState()`. The scaffold is a starting point, not a constraint.

## How `load_shell()` Works

At startup, Pulpkit resolves the shell directory and loads files in this order:

1. **Resolve directory** — `poc/shells/{name}/` relative to the project root
2. **Load bar.html** — required; falls back to built-in default if missing
3. **Load popup.html** — required; falls back to built-in default if missing
4. **Load toast.html** — optional; falls back to `<html><body></body></html>`
5. **Parse config.json** — optional; falls back to default values

```
pulpkit zenith          # loads poc/shells/zenith/
pulpkit                 # loads built-in default (poc/src/bar.html, poc/src/popup.html)
```

The loaded HTML is injected into WebKitGTK webviews via `load_html()`. Component
scripts referenced via `<script src="...">` are resolved relative to the shell
directory because the base URI is set to `file:///`.

!!! warning "File protocol"
    WebView content is loaded with a `file:///` base URI. Relative paths in
    `<script src>` and `<link href>` resolve against the shell directory.
    Do not use absolute URLs unless fetching remote resources.

## Hot Reloading

During development, use the MCP server to reload without restarting:

```
hot_reload_bar(path: "/path/to/shells/my-shell/bar.html")
hot_reload_popup(path: "/path/to/shells/my-shell/popup.html")
```

This replaces the webview content in-place and triggers a fresh `updateState()` call
with current system state.
