# Lock Screen

Pulpkit supports custom lock screens rendered as a WebKitGTK webview. The lock screen
uses PAM for password verification and the Wayland `ext-session-lock-v1` protocol
to secure the session.

## lock.html Contract

The lock screen file follows the same `updateState(s)` / `send(o)` contract:

- Must define `function updateState(s)`
- Must define `function send(o)`
- Rendered as a fullscreen layer-shell surface when the session is locked
- Must handle password input and the PAM authentication flow
- Must call `send({cmd: 'unlock'})` after successful authentication

## PAM Authentication Flow

The lock screen authenticates through a three-step exchange with the Rust backend:

```
┌──────────────┐     verify_password     ┌──────────────┐
│  lock.html   │ ──────────────────────→ │    Pulpkit    │
│  (WebView)   │                         │   (PAM call)  │
│              │ ←────────────────────── │              │
│              │  s.custom.auth_result   │              │
│              │                         │              │
│              │     unlock              │              │
│              │ ──────────────────────→ │  loginctl     │
└──────────────┘                         │  unlock-session│
                                         └──────────────┘
```

### Step 1: Submit Password

When the user types their password and presses Enter:

```js
send({cmd: 'verify_password', data: passwordInput.value});
```

### Step 2: Check Result

The Rust backend verifies the password via PAM and sets `s.custom.auth_result`:

- `s.custom.auth_result === true` — password correct
- `s.custom.auth_result === false` — password incorrect

```js
function updateState(s) {
  if (s.custom && s.custom.auth_result === true) {
    // Password correct — unlock
    send({cmd: 'unlock'});
  } else if (s.custom && s.custom.auth_result === false) {
    // Password wrong — show error
    showError('Incorrect password');
    passwordInput.value = '';
    passwordInput.focus();
  }
}
```

### Step 3: Unlock

The `unlock` command calls `loginctl unlock-session` and clears the `auth_result`
custom key. The session lock surface is then dismissed by the compositor.

## Session Lock Protocol

Pulpkit uses the `ext-session-lock-v1` Wayland protocol:

1. **Lock trigger** — `send({cmd: 'power_lock'})` or `loginctl lock-session`
2. **Compositor locks** — the compositor activates session lock, covering all outputs
3. **Lock surface** — Pulpkit renders `lock.html` as a session lock surface
4. **Authentication** — user authenticates through the `verify_password` flow
5. **Unlock** — `loginctl unlock-session` releases the lock

!!! warning "Security"
    The `ext-session-lock-v1` protocol ensures no other surfaces can be interacted
    with while locked. The lock surface covers all outputs. The compositor will not
    unlock until the client explicitly requests it.

## Monitoring Lock State

The state object includes session fields useful for lock screens:

| Field | Description |
|---|---|
| `s.session_locked` | `true` when the session is locked |
| `s.session_idle` | `true` when the session is idle |
| `s.preparing_sleep` | `true` when the system is about to suspend |

```js
function updateState(s) {
  if (s.preparing_sleep) {
    // System is about to sleep — show a sleep indicator
  }
}
```

## Design Considerations

### Keyboard Focus

The lock screen must handle keyboard input. Ensure the password field is focused
on load:

```js
window.addEventListener('load', () => {
  document.getElementById('password').focus();
});

// Re-focus on any click
document.addEventListener('click', () => {
  document.getElementById('password').focus();
});
```

### Clock and Date

Lock screens typically show the time. Since `updateState()` is called every 80ms,
you can use a `setInterval` for the clock or derive it from state updates:

```js
function updateClock() {
  const now = new Date();
  setText('time', now.toLocaleTimeString([], {hour: '2-digit', minute: '2-digit'}));
  setText('date', now.toLocaleDateString([], {weekday: 'long', month: 'long', day: 'numeric'}));
}
setInterval(updateClock, 1000);
updateClock();
```

### User Info

Display user information from state:

```js
function updateState(s) {
  setText('username', s.user);
  setText('hostname', s.host);
  if (s.user_icon) {
    document.getElementById('avatar').src = s.user_icon;
  }
}
```

### Error Feedback

Provide clear visual feedback for incorrect passwords:

```css
@keyframes shake {
  0%, 100% { transform: translateX(0); }
  25% { transform: translateX(-8px); }
  75% { transform: translateX(8px); }
}

.password-error {
  animation: shake 0.3s ease-in-out;
  border-color: var(--red) !important;
}
```

```js
function showError(msg) {
  const input = document.getElementById('password');
  input.classList.add('password-error');
  setTimeout(() => input.classList.remove('password-error'), 500);
}
```

### Media Information

Show currently playing media on the lock screen:

```js
function updateState(s) {
  const mediaEl = document.getElementById('media');
  if (s.media_playing) {
    mediaEl.style.display = 'flex';
    setText('media-title', s.media_title);
    setText('media-artist', s.media_artist);
  } else {
    mediaEl.style.display = 'none';
  }
}
```

### Battery Warning

Show battery status on the lock screen, especially if low:

```js
function updateState(s) {
  if (s.has_bat && s.bat < 15 && s.bat_status === 'Discharging') {
    document.getElementById('battery-warn').style.display = 'block';
    setText('battery-pct', s.bat + '%');
  }
}
```

## Example: Minimal Lock Screen

```html
<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<style>
  * { margin: 0; padding: 0; box-sizing: border-box; }
  body {
    font-family: 'JetBrainsMono Nerd Font', monospace;
    background: #181825;
    color: #cdd6f4;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100vh;
    gap: 24px;
  }
  #time { font-size: 64px; font-weight: 200; }
  #date { font-size: 16px; color: #a6adc8; }
  #password {
    width: 300px; padding: 12px 20px;
    background: #1e1e2e; border: 2px solid #313244;
    border-radius: 24px; color: #cdd6f4;
    font-size: 16px; text-align: center;
    outline: none;
  }
  #password:focus { border-color: #cba6f7; }
  #error { color: #f38ba8; font-size: 13px; min-height: 20px; }
</style>
</head>
<body>
<div id="time"></div>
<div id="date"></div>
<input id="password" type="password" placeholder="Password" autofocus>
<div id="error"></div>

<script>
function send(o) {
  window.webkit.messageHandlers.pulpkit.postMessage(JSON.stringify(o));
}

function setText(id, t) {
  const e = document.getElementById(id);
  if (e && e.textContent !== t) e.textContent = t;
}

// Clock
setInterval(() => {
  const now = new Date();
  setText('time', now.toLocaleTimeString([], {hour:'2-digit', minute:'2-digit'}));
  setText('date', now.toLocaleDateString([], {weekday:'long', month:'long', day:'numeric'}));
}, 1000);

// Password submit
document.getElementById('password').addEventListener('keydown', e => {
  if (e.key === 'Enter') {
    send({cmd: 'verify_password', data: e.target.value});
  }
});

function updateState(s) {
  if (s.custom && s.custom.auth_result === true) {
    send({cmd: 'unlock'});
  } else if (s.custom && s.custom.auth_result === false) {
    document.getElementById('error').textContent = 'Incorrect password';
    const pw = document.getElementById('password');
    pw.value = '';
    pw.focus();
  }
}
</script>
</body>
</html>
```
