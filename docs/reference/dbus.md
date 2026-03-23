# DBus Services Reference

Pulpkit exposes DBus services on the session bus for external tool integration and
implements the standard XDG notification daemon specification.

## org.pulpkit.Shell

External tools can control the shell over DBus.

| | |
|---|---|
| **Bus** | Session bus |
| **Service name** | `org.pulpkit.Shell` |
| **Object path** | `/org/pulpkit/Shell` |

### Methods

#### `GetState`

Returns the full serialized state as a JSON string.

| | |
|---|---|
| **Arguments** | None |
| **Returns** | `string` (JSON) |

```bash
busctl --user call org.pulpkit.Shell /org/pulpkit/Shell \
  org.pulpkit.Shell GetState
```

```python
# Python example
import dbus
bus = dbus.SessionBus()
proxy = bus.get_object('org.pulpkit.Shell', '/org/pulpkit/Shell')
iface = dbus.Interface(proxy, 'org.pulpkit.Shell')
state_json = iface.GetState()
```

---

#### `SetCustom`

Set a custom state key-value pair. The value appears in `s.custom` on the next
state push.

| | |
|---|---|
| **Arguments** | `key: string`, `value: string` |
| **Returns** | None |

```bash
busctl --user call org.pulpkit.Shell /org/pulpkit/Shell \
  org.pulpkit.Shell SetCustom ss "my_key" "my_value"
```

---

#### `TogglePopup`

Toggle a popup panel by name. If the named panel is already open, it closes.

| | |
|---|---|
| **Arguments** | `name: string` |
| **Returns** | None |

```bash
busctl --user call org.pulpkit.Shell /org/pulpkit/Shell \
  org.pulpkit.Shell TogglePopup s "settings"
```

---

#### `Dismiss`

Close any open popup.

| | |
|---|---|
| **Arguments** | None |
| **Returns** | None |

```bash
busctl --user call org.pulpkit.Shell /org/pulpkit/Shell \
  org.pulpkit.Shell Dismiss
```

---

#### `Exec`

Run a shell command.

| | |
|---|---|
| **Arguments** | `command: string` |
| **Returns** | None |

```bash
busctl --user call org.pulpkit.Shell /org/pulpkit/Shell \
  org.pulpkit.Shell Exec s "notify-send 'Hello from DBus'"
```

!!! warning "Security"
    The `Exec` method runs arbitrary commands as the session user. Only expose this
    on the session bus (not the system bus).

## org.freedesktop.Notifications

Pulpkit implements the [XDG Desktop Notifications Specification](https://specifications.freedesktop.org/notification-spec/notification-spec-latest.html).
It acts as the notification daemon, receiving notifications from applications and
storing them in `s.notifications`.

| | |
|---|---|
| **Bus** | Session bus |
| **Service name** | `org.freedesktop.Notifications` |
| **Object path** | `/org/freedesktop/Notifications` |

### Methods

#### `Notify`

Receive and display a notification. Standard XDG Notify signature.

| | |
|---|---|
| **Arguments** | `app_name: string`, `replaces_id: u32`, `app_icon: string`, `summary: string`, `body: string`, `actions: string[]`, `hints: dict`, `expire_timeout: i32` |
| **Returns** | `u32` (notification ID) |

```bash
# Send a test notification
notify-send "Test" "Hello from Pulpkit"

# Or via DBus directly
busctl --user call org.freedesktop.Notifications \
  /org/freedesktop/Notifications \
  org.freedesktop.Notifications Notify \
  susssasa\{sv\}i \
  "test-app" 0 "" "Summary" "Body text" 0 0 5000
```

Received notifications are added to:

- `s.notifications[]` — full notification objects
- `s.notif_count` — incremented count

---

#### `CloseNotification`

Close a notification by ID.

| | |
|---|---|
| **Arguments** | `id: u32` |
| **Returns** | None |

---

#### `GetCapabilities`

Returns the list of supported capabilities.

| | |
|---|---|
| **Arguments** | None |
| **Returns** | `string[]` |

---

#### `GetServerInformation`

Returns information about the notification daemon.

| | |
|---|---|
| **Arguments** | None |
| **Returns** | `name: string`, `vendor: string`, `version: string`, `spec_version: string` |

Returns: `("Pulpkit", "pulpkit", "0.1.0", "1.2")`

## Integration Examples

### Toggle popup from a keybinding

In your compositor config (e.g., niri):

```kdl
binds {
  Mod+S { spawn "busctl" "--user" "call" "org.pulpkit.Shell" "/org/pulpkit/Shell" "org.pulpkit.Shell" "TogglePopup" "s" "settings"; }
  Mod+W { spawn "busctl" "--user" "call" "org.pulpkit.Shell" "/org/pulpkit/Shell" "org.pulpkit.Shell" "TogglePopup" "s" "wifi"; }
}
```

### Set custom state from a script

```bash
#!/bin/bash
# Set a custom notification badge
busctl --user call org.pulpkit.Shell /org/pulpkit/Shell \
  org.pulpkit.Shell SetCustom ss "mail_count" "5"
```

### Read state from a script

```bash
#!/bin/bash
state=$(busctl --user call org.pulpkit.Shell /org/pulpkit/Shell \
  org.pulpkit.Shell GetState | jq -r '.vol')
echo "Volume: $state"
```
