# Commands

Shells communicate with the Pulpkit runtime by sending commands from JavaScript.
Commands control system settings, navigate workspaces, launch applications, and
manage the shell UI.

## The `send()` Function

Every shell HTML file must define this helper:

```js
function send(o) {
  window.webkit.messageHandlers.pulpkit.postMessage(JSON.stringify(o));
}
```

Commands are objects with a `cmd` field and an optional `data` field:

```js
send({ cmd: 'vol_set', data: 75 });
send({ cmd: 'dismiss' });  // no data needed
```

The runtime parses the JSON, dispatches on `cmd`, and executes the corresponding
system action.

## Command Reference

### Audio

| Command | Data | Description |
|---|---|---|
| `vol_set` | `number` (0-100) | Set audio volume |
| `vol_mute` | -- | Toggle mute on default audio sink |

```js
// Set volume to 50%
send({cmd: 'vol_set', data: 50});

// Toggle mute
send({cmd: 'vol_mute'});
```

### Display

| Command | Data | Description |
|---|---|---|
| `bri_set` | `number` (0-100) | Set screen brightness |
| `toggle_night` | -- | Toggle night light (wlsunset) |

```js
// Set brightness to 80%
send({cmd: 'bri_set', data: 80});

// Toggle night light
send({cmd: 'toggle_night'});
```

### Workspaces & Windows

| Command | Data | Description |
|---|---|---|
| `ws_go` | `number` | Switch to workspace by index |
| `focus_window` | `number` | Focus window by ID |
| `close_window` | `number` | Close window by ID |
| `move_to_workspace` | `{id, ws}` | Move window to workspace |

```js
// Switch to workspace 3
send({cmd: 'ws_go', data: 3});

// Focus window with id 42
send({cmd: 'focus_window', data: 42});

// Close window with id 42
send({cmd: 'close_window', data: 42});

// Move window 42 to workspace 2
send({cmd: 'move_to_workspace', data: {id: 42, ws: 2}});
```

### Network

| Command | Data | Description |
|---|---|---|
| `wifi_con` | `string` (SSID) | Connect to WiFi network |
| `wifi_dis` | -- | Disconnect WiFi |

```js
// Connect to a network
send({cmd: 'wifi_con', data: 'HomeNetwork'});

// Disconnect
send({cmd: 'wifi_dis'});
```

### Bluetooth

| Command | Data | Description |
|---|---|---|
| `toggle_bt` | -- | Toggle Bluetooth power |

```js
send({cmd: 'toggle_bt'});
```

### Notifications

| Command | Data | Description |
|---|---|---|
| `toggle_dnd` | -- | Toggle Do Not Disturb mode |
| `notif_dismiss` | -- | Dismiss all notifications (mako) |
| `notif_dismiss_one` | -- | Dismiss latest notification (mako) |
| `notif_clear_all` | -- | Clear all notifications from state |

```js
// Toggle DND
send({cmd: 'toggle_dnd'});

// Dismiss all notifications via mako
send({cmd: 'notif_dismiss'});

// Clear notification list from Pulpkit state
send({cmd: 'notif_clear_all'});
```

!!! note "`notif_dismiss` vs `notif_clear_all`"
    `notif_dismiss` tells mako to dismiss notifications. `notif_clear_all` clears the
    `s.notifications` array and resets `s.notif_count` in Pulpkit's internal state.
    You typically want both.

### Power & Session

| Command | Data | Description |
|---|---|---|
| `set_profile` | `string` | Set power profile: `"balanced"`, `"performance"`, or `"power-saver"` |
| `power_lock` | -- | Lock the session |
| `power_suspend` | -- | Suspend the system |
| `power_reboot` | -- | Reboot the system |
| `power_shutdown` | -- | Shut down the system |
| `power_logout` | -- | Log out (niri quit) |

```js
// Set power profile
send({cmd: 'set_profile', data: 'performance'});

// Lock screen
send({cmd: 'power_lock'});

// Reboot
send({cmd: 'power_reboot'});
```

### Applications

| Command | Data | Description |
|---|---|---|
| `launch` | `string` | Launch application by exec command |
| `exec` | `string` | Run arbitrary shell command |

```js
// Launch Firefox
send({cmd: 'launch', data: 'firefox'});

// Run a shell command
send({cmd: 'exec', data: 'notify-send "Hello"'});
```

!!! warning "launch vs exec"
    `launch` also dismisses the popup after launching. Use it for app launcher panels.
    `exec` runs the command silently without UI side effects.

### Screenshots

| Command | Data | Description |
|---|---|---|
| `screenshot` | -- | Region screenshot (grim + slurp), copied to clipboard |
| `screenshot_full` | -- | Full screen screenshot, copied to clipboard |

```js
send({cmd: 'screenshot'});       // interactive region select
send({cmd: 'screenshot_full'});  // full screen
```

### UI Control

| Command | Data | Description |
|---|---|---|
| `popup` | `string` | Toggle a popup panel by name |
| `dismiss` | -- | Close any open popup |

```js
// Open settings panel (or close if already open)
send({cmd: 'popup', data: 'settings'});

// Close whatever popup is open
send({cmd: 'dismiss'});
```

The `popup` command toggles: if the named panel is already open, it closes. Standard
panel names are `settings`, `wifi`, `power`, `launcher`, and `config`, but you can
use any string for custom panels.

!!! tip "Special popup behavior"
    Opening `wifi` triggers a WiFi network scan. Opening `launcher` scans installed
    applications. The results appear in `s.wifi_nets` and `s.apps` on the next state push.

### Theming

| Command | Data | Description |
|---|---|---|
| `set_theme` | `string` | Switch color theme by name |

```js
send({cmd: 'set_theme', data: 'tokyonight'});
```

Available themes: `mocha`, `macchiato`, `frappe`, `latte`, `tokyonight`, `nord`,
`gruvbox`, `rosepine`, `onedark`, `dracula`, `solarized`, `flexoki`.

### Custom State

| Command | Data | Description |
|---|---|---|
| `set_custom` | `{key, value}` | Store a custom key-value pair in state |

```js
// Set a custom value
send({cmd: 'set_custom', data: {key: 'sidebar_open', value: true}});

// Read it back in updateState:
// s.custom.sidebar_open === true
```

### System Tray

| Command | Data | Description |
|---|---|---|
| `tray_activate` | `{address, click}` | Activate a system tray item |

```js
// Left-click a tray item
send({cmd: 'tray_activate', data: {address: ':1.100', click: 'left'}});

// Right-click a tray item
send({cmd: 'tray_activate', data: {address: ':1.100', click: 'right'}});
```

The `address` comes from `s.tray_items[].address`.

### Lock Screen Authentication

| Command | Data | Description |
|---|---|---|
| `verify_password` | `string` | Verify password via PAM |
| `unlock` | -- | Unlock the session |

```js
// Submit password for verification
send({cmd: 'verify_password', data: passwordInput.value});

// After s.custom.auth_result === true, unlock
send({cmd: 'unlock'});
```

See the [lock screen guide](lock-screen.md) for the full authentication flow.
