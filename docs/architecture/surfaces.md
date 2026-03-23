# Surfaces

Pulpkit creates 4 primary surfaces (plus 1 backdrop helper) using the Wayland layer-shell protocol. Each surface is a GTK4 window with an embedded WebKitGTK `WebView` that renders HTML/CSS/JS.

---

## Bar

The bar is the persistent desktop panel -- always visible, always on top of normal windows.

| Property | Value |
|----------|-------|
| **Layer** | `Layer::Top` |
| **Keyboard mode** | `KeyboardMode::None` |
| **Anchors** | Left + Right + Top (or Bottom if `bar_position = "bottom"`) |
| **Exclusive zone** | Auto (reserves space so windows don't overlap the bar) |
| **Background** | Transparent (CSS: `window { background: transparent; }`) |
| **Default height** | 40px (configurable via `config.json`) |

### Behavior

- Spans the full width of the output
- Reserves exclusive zone so tiled windows and popups avoid the bar area
- Receives state pushes on every render cycle
- Sends commands to Rust via `window.webkit.messageHandlers.pulpkit.postMessage()`
- Does not accept keyboard input (clicks only)

### HTML contract

The bar HTML must define a global `updateState(state)` function:

```javascript
function updateState(state) {
  // state is the full FullState JSON object
  // Update DOM elements based on state fields
}
```

---

## Popup

The popup is a centered overlay panel for settings, launcher, WiFi selector, power menu, and other contextual UI.

| Property | Value |
|----------|-------|
| **Layer** | `Layer::Overlay` |
| **Keyboard mode** | `KeyboardMode::OnDemand` |
| **Anchors** | None (centered by compositor) |
| **Exclusive zone** | None |
| **Background** | Transparent |
| **Default size** | 380x500px (configurable via `config.json`) |

### Behavior

- Hidden by default (`set_visible(false)`)
- Shown when `state.popup` is non-empty, hidden when empty
- Receives state pushes only while visible
- Sits above everything including the bar (Overlay layer)
- Accepts keyboard input on demand (for text entry in launcher, WiFi password, etc.)

### Backdrop

When the popup is visible, a **backdrop surface** covers the entire screen behind it:

| Property | Value |
|----------|-------|
| **Layer** | `Layer::Overlay` |
| **Keyboard mode** | `KeyboardMode::None` |
| **Anchors** | All edges (full screen) |
| **Background** | Configurable: `"dim"` (default, `rgba(0,0,0,0.25)`), `"none"` (transparent), or custom |

Clicking the backdrop dismisses the popup by clearing `state.popup`. The backdrop can be disabled entirely by setting `popup_backdrop = "none"` in the theme's `config.json`.

### Popup names

The `popup` field in state determines which view the popup HTML renders. Common values:

| Value | Purpose |
|-------|---------|
| `"settings"` | Quick settings panel |
| `"wifi"` | WiFi network selector |
| `"launcher"` | Application launcher |
| `"power"` | Power / session menu |
| `"bluetooth"` | Bluetooth device manager |
| `"audio"` | Audio device and stream mixer |
| `"calendar"` | Calendar and events |
| (empty string) | Popup is hidden |

The popup HTML is responsible for switching its own content based on `state.popup`.

### HTML contract

Same as the bar -- define `updateState(state)`:

```javascript
function updateState(state) {
  if (state.popup === "settings") {
    // Render settings panel
  } else if (state.popup === "launcher") {
    // Render app launcher
  }
  // ...
}
```

---

## Toast

The toast surface displays notification popups in the top-right corner. Notifications are received by the built-in notification daemon (`org.freedesktop.Notifications`) and pushed to the toast WebView via state.

| Property | Value |
|----------|-------|
| **Layer** | `Layer::Top` |
| **Keyboard mode** | `KeyboardMode::None` |
| **Anchors** | Top + Right |
| **Exclusive zone** | None |
| **Background** | Transparent |
| **Default size** | 380x300px |
| **Margins** | Top: 2px, Right: 8px |

### Behavior

- Always visible (presented at startup) but renders transparently when there are no notifications
- Does not accept input -- notifications auto-expire or are dismissed via commands from other surfaces
- Receives state pushes on every render cycle (including `notifications` array)
- Sits on `Layer::Top`, same as the bar, but anchored to the top-right corner
- No exclusive zone, so it overlaps window content

### HTML contract

```javascript
function updateState(state) {
  // state.notifications is an array of Notification objects
  // Render notification stack
  // Each notification has: id, app_name, summary, body, icon, timestamp
}
```

!!! tip "Pass-through input"
    Since the toast has `KeyboardMode::None` and no explicit input handling, pointer events pass through to windows below when the toast is not rendering any visible content. The transparent background ensures it does not block interaction.

---

## Lock

The lock surface is a full-screen overlay that covers all outputs when the session is locked. It renders a password/PIN entry form and uses PAM for authentication.

| Property | Value |
|----------|-------|
| **Layer** | `Layer::Overlay` |
| **Keyboard mode** | `KeyboardMode::Exclusive` |
| **Anchors** | All edges (full screen) |
| **Exclusive zone** | None |
| **Background** | Opaque (theme-defined) |

### Behavior

- Created on session lock events (from logind `LockedHint`)
- Covers the entire screen on all outputs
- Grabs exclusive keyboard input so no other surface can receive keystrokes
- The HTML form sends `verify_password` commands with the entered password
- Rust side calls `pam::verify_password()` and stores the result in `custom["auth_result"]`
- On successful authentication, the HTML sends an `unlock` command which calls `loginctl unlock-session`

### Security model

- PAM verification runs in-process (no shell-out to passwd)
- The lock surface is on `Layer::Overlay` with `KeyboardMode::Exclusive`, preventing input to any other surface
- The `ext-session-lock` protocol ensures the compositor does not render unlocked content

### HTML contract

```javascript
function updateState(state) {
  // state.session_locked: whether the session is locked
  // state.custom.auth_result: true/false after password verification
  // Render lock screen with time, date, password input
}

function submitPassword(password) {
  window.webkit.messageHandlers.pulpkit.postMessage(
    JSON.stringify({ cmd: "verify_password", data: password })
  );
}

function unlock() {
  window.webkit.messageHandlers.pulpkit.postMessage(
    JSON.stringify({ cmd: "unlock" })
  );
}
```

---

## State Push Flow

All surfaces receive state through the same mechanism -- JavaScript evaluation on the main GTK thread:

```
80ms timer fires
  |
  v
dirty flag set? ---- no ----> return
  |
  yes
  |
  v
Lock FullState, merge AppState, serialize to JSON
  |
  v
JSON === last_json? ---- yes ----> return
  |
  no
  |
  v
Build JS: "if(typeof updateState==='function')updateState({...})"
  |
  +---> bar_wv.evaluate_javascript(script)      [always]
  +---> toast_wv.evaluate_javascript(script)     [always]
  +---> popup_wv.evaluate_javascript(script)     [only if popup visible]
```

!!! note "Popup optimization"
    The popup WebView only receives state pushes when `state.popup` is non-empty. This avoids unnecessary JS evaluation in a hidden surface.

---

## Surface Stacking Order

From bottom to top:

```
1. Normal application windows
2. Bar          (Layer::Top)       ─── always visible
3. Toast        (Layer::Top)       ─── always visible (transparent when empty)
4. Backdrop     (Layer::Overlay)   ─── visible only when popup is open
5. Popup        (Layer::Overlay)   ─── visible only when popup is open
6. Lock         (Layer::Overlay)   ─── visible only when session is locked
```

The `Layer::Overlay` surfaces always render above `Layer::Top` surfaces. Within the same layer, the compositor determines stacking order (typically most recently mapped on top).
