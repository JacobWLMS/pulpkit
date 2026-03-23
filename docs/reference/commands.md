# Command Reference

Complete reference for every command available via `send({cmd, data})`.

## Sending Commands

```js
function send(o) {
  window.webkit.messageHandlers.pulpkit.postMessage(JSON.stringify(o));
}

// Usage
send({cmd: 'vol_set', data: 75});
send({cmd: 'dismiss'});  // no data field needed
```

## All Commands

### Audio

| Command | Data Type | Description | Example |
|---|---|---|---|
| `vol_set` | `number` (0-100) | Set audio volume | `send({cmd:'vol_set', data:75})` |
| `vol_mute` | -- | Toggle mute on default sink | `send({cmd:'vol_mute'})` |

### Display

| Command | Data Type | Description | Example |
|---|---|---|---|
| `bri_set` | `number` (0-100) | Set screen brightness | `send({cmd:'bri_set', data:80})` |
| `toggle_night` | -- | Toggle night light (wlsunset) | `send({cmd:'toggle_night'})` |

### Network

| Command | Data Type | Description | Example |
|---|---|---|---|
| `wifi_con` | `string` (SSID) | Connect to WiFi network | `send({cmd:'wifi_con', data:'MyNetwork'})` |
| `wifi_dis` | -- | Disconnect WiFi | `send({cmd:'wifi_dis'})` |

### Bluetooth

| Command | Data Type | Description | Example |
|---|---|---|---|
| `toggle_bt` | -- | Toggle Bluetooth power | `send({cmd:'toggle_bt'})` |

### Workspaces & Windows

| Command | Data Type | Description | Example |
|---|---|---|---|
| `ws_go` | `number` | Switch workspace by index | `send({cmd:'ws_go', data:3})` |
| `focus_window` | `number` (window ID) | Focus a window | `send({cmd:'focus_window', data:42})` |
| `close_window` | `number` (window ID) | Close a window | `send({cmd:'close_window', data:42})` |
| `move_to_workspace` | `{id, ws}` | Move window to workspace | `send({cmd:'move_to_workspace', data:{id:42, ws:2}})` |

### Notifications

| Command | Data Type | Description | Example |
|---|---|---|---|
| `toggle_dnd` | -- | Toggle Do Not Disturb | `send({cmd:'toggle_dnd'})` |
| `notif_dismiss` | -- | Dismiss all (mako) | `send({cmd:'notif_dismiss'})` |
| `notif_dismiss_one` | -- | Dismiss latest (mako) | `send({cmd:'notif_dismiss_one'})` |
| `notif_clear_all` | -- | Clear all from state | `send({cmd:'notif_clear_all'})` |

### Power & Session

| Command | Data Type | Description | Example |
|---|---|---|---|
| `set_profile` | `string` | Set power profile | `send({cmd:'set_profile', data:'performance'})` |
| `power_lock` | -- | Lock session | `send({cmd:'power_lock'})` |
| `power_suspend` | -- | Suspend system | `send({cmd:'power_suspend'})` |
| `power_reboot` | -- | Reboot system | `send({cmd:'power_reboot'})` |
| `power_shutdown` | -- | Shut down system | `send({cmd:'power_shutdown'})` |
| `power_logout` | -- | Log out (niri quit) | `send({cmd:'power_logout'})` |

Valid power profiles: `"balanced"`, `"performance"`, `"power-saver"`.

### Applications

| Command | Data Type | Description | Example |
|---|---|---|---|
| `launch` | `string` (exec command) | Launch app and dismiss popup | `send({cmd:'launch', data:'firefox'})` |
| `exec` | `string` (shell command) | Run shell command silently | `send({cmd:'exec', data:'notify-send Hi'})` |

### Screenshots

| Command | Data Type | Description | Example |
|---|---|---|---|
| `screenshot` | -- | Region screenshot (grim+slurp) | `send({cmd:'screenshot'})` |
| `screenshot_full` | -- | Full screen screenshot | `send({cmd:'screenshot_full'})` |

Both commands copy the screenshot to the clipboard via `wl-copy`.

### UI Control

| Command | Data Type | Description | Example |
|---|---|---|---|
| `popup` | `string` (panel name) | Toggle popup panel | `send({cmd:'popup', data:'settings'})` |
| `dismiss` | -- | Close any open popup | `send({cmd:'dismiss'})` |

Standard panel names: `settings`, `wifi`, `power`, `launcher`, `config`. Custom panel
names are also supported.

### Theming

| Command | Data Type | Description | Example |
|---|---|---|---|
| `set_theme` | `string` (theme name) | Switch color theme | `send({cmd:'set_theme', data:'nord'})` |

Available themes: `mocha`, `macchiato`, `frappe`, `latte`, `tokyonight`, `nord`,
`gruvbox`, `rosepine`, `onedark`, `dracula`, `solarized`, `flexoki`.

### Custom State

| Command | Data Type | Description | Example |
|---|---|---|---|
| `set_custom` | `{key, value}` | Store custom key-value pair | `send({cmd:'set_custom', data:{key:'foo', value:42}})` |

The value can be any JSON type (string, number, boolean, object, array, null).

### System Tray

| Command | Data Type | Description | Example |
|---|---|---|---|
| `tray_activate` | `{address, click}` | Activate tray item | `send({cmd:'tray_activate', data:{address:':1.100', click:'left'}})` |

`click` must be `"left"` or `"right"`. The `address` comes from `s.tray_items[].address`.

### Lock Screen Authentication

| Command | Data Type | Description | Example |
|---|---|---|---|
| `verify_password` | `string` | Verify password via PAM | `send({cmd:'verify_password', data:'mypassword'})` |
| `unlock` | -- | Unlock session | `send({cmd:'unlock'})` |

The `verify_password` command sets `s.custom.auth_result` to `true` or `false`.
After confirming `auth_result === true`, call `unlock` to release the session lock.
