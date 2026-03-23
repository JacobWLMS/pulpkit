# Toast Notifications

Toast notifications are transient popups that appear when your system receives
desktop notifications via DBus. Pulpkit runs a built-in notification daemon that
captures notifications and pushes them to the toast webview.

## toast.html Contract

The toast file follows the same contract as bar.html:

- Must define `function updateState(s)` — receives the full state including notifications
- Must define `function send(o)` — sends commands to the Rust backend
- Rendered in a layer-shell surface anchored to the **top-right** corner
- Surface dimensions: 380x300 pixels
- Background is transparent (the webview has `rgba(0,0,0,0)` background)

```html
<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <link rel="stylesheet" href="theme.css">
  <style>
    * { margin: 0; padding: 0; box-sizing: border-box; }
    body {
      font-family: 'JetBrainsMono Nerd Font', monospace;
      background: transparent;
      overflow: hidden;
    }
  </style>
</head>
<body>
<div id="toasts"></div>
<script>
function send(o) {
  window.webkit.messageHandlers.pulpkit.postMessage(JSON.stringify(o));
}

function updateState(s) {
  if (s.theme) document.documentElement.setAttribute('data-theme', s.theme);
  renderToasts(s.notifications);
}
</script>
</body>
</html>
```

## Notification Flow

```
Application → DBus (org.freedesktop.Notifications.Notify)
           → Pulpkit notification daemon
           → FullState.notifications array updated
           → Dirty flag set
           → updateState(s) called in toast webview (and bar/popup)
```

1. An application sends a `Notify` call over DBus
2. Pulpkit's built-in notification daemon (implementing the `org.freedesktop.Notifications` spec) receives it
3. The notification is added to `s.notifications` and `s.notif_count` is incremented
4. On the next 80ms tick, `updateState(s)` is called in all webviews
5. The toast webview renders the notification

## Notification Object Shape

Each item in `s.notifications` has this shape:

```js
{
  "id": 1,              // u32 — unique notification ID
  "app_name": "Discord", // string — sending application name
  "summary": "New Message", // string — notification title
  "body": "Hey, are you there?", // string — notification body text
  "icon": "",           // string — icon path or name
  "timestamp": 1711234567 // u64 — Unix timestamp when received
}
```

| Field | Type | Description |
|---|---|---|
| `id` | `number` | Unique notification ID assigned by the daemon |
| `app_name` | `string` | Name of the application that sent the notification |
| `summary` | `string` | Notification title/heading |
| `body` | `string` | Notification body text (may contain basic HTML) |
| `icon` | `string` | Icon path or freedesktop icon name |
| `timestamp` | `number` | Unix timestamp (seconds since epoch) |

## Animation Patterns

### Slide-in Animation

New notifications should animate in from the top or right:

```css
@keyframes slideIn {
  from { transform: translateX(100%); opacity: 0; }
  to   { transform: translateX(0);    opacity: 1; }
}

.toast {
  animation: slideIn 0.2s ease-out;
}
```

### Slide-out on Dismiss

```css
@keyframes slideOut {
  from { transform: translateX(0);    opacity: 1; }
  to   { transform: translateX(100%); opacity: 0; }
}

.toast.dismissing {
  animation: slideOut 0.2s ease-in forwards;
}
```

## Auto-Dismiss

The runtime does not auto-dismiss notifications from state. Implement auto-dismiss
in your toast.html with JavaScript timers:

```js
const TOAST_TIMEOUT = 5000; // 5 seconds
const activeToasts = new Map();

function renderToasts(notifications) {
  const container = document.getElementById('toasts');

  notifications.forEach(n => {
    // Skip if already showing
    if (activeToasts.has(n.id)) return;

    const el = document.createElement('div');
    el.className = 'toast';
    el.innerHTML = `
      <div class="toast-header">
        <span class="toast-app">${n.app_name}</span>
      </div>
      <div class="toast-summary">${n.summary}</div>
      ${n.body ? `<div class="toast-body">${n.body}</div>` : ''}
    `;
    container.appendChild(el);
    activeToasts.set(n.id, el);

    // Auto-dismiss after timeout
    setTimeout(() => {
      el.classList.add('dismissing');
      setTimeout(() => {
        el.remove();
        activeToasts.delete(n.id);
      }, 200); // match animation duration
    }, TOAST_TIMEOUT);
  });
}
```

!!! tip "DND mode"
    Check `s.dnd` before showing toasts. When Do Not Disturb is active, you may
    want to suppress toast rendering:

    ```js
    function updateState(s) {
      if (s.dnd) return; // suppress toasts in DND mode
      renderToasts(s.notifications);
    }
    ```

## Clearing Notifications

To clear all notifications from state (e.g., a "Clear all" button in the popup):

```js
send({cmd: 'notif_clear_all'});
```

This empties `s.notifications` and resets `s.notif_count` to 0.

To dismiss notifications via mako (the external notification daemon):

```js
send({cmd: 'notif_dismiss'});      // dismiss all
send({cmd: 'notif_dismiss_one'});  // dismiss latest
```

## Example: Minimal Toast

```html
<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<style>
  * { margin: 0; padding: 0; box-sizing: border-box; }
  body { background: transparent; font-family: system-ui; }

  .toast {
    background: rgba(30, 30, 46, 0.95);
    border: 1px solid rgba(255,255,255,0.1);
    border-radius: 12px;
    padding: 12px 16px;
    margin-bottom: 8px;
    color: #cdd6f4;
    animation: slideIn 0.2s ease-out;
    max-width: 360px;
  }
  .toast-app { font-size: 11px; color: #a6adc8; }
  .toast-summary { font-size: 13px; font-weight: 600; margin-top: 2px; }
  .toast-body { font-size: 12px; color: #a6adc8; margin-top: 4px; }

  @keyframes slideIn {
    from { transform: translateX(100%); opacity: 0; }
    to   { transform: translateX(0);    opacity: 1; }
  }
</style>
</head>
<body>
<div id="toasts"></div>
<script>
function send(o) {
  window.webkit.messageHandlers.pulpkit.postMessage(JSON.stringify(o));
}

const shown = new Set();

function updateState(s) {
  if (s.dnd) return;
  const container = document.getElementById('toasts');
  (s.notifications || []).forEach(n => {
    if (shown.has(n.id)) return;
    shown.add(n.id);
    const el = document.createElement('div');
    el.className = 'toast';
    el.innerHTML = `
      <div class="toast-app">${n.app_name}</div>
      <div class="toast-summary">${n.summary}</div>
      ${n.body ? '<div class="toast-body">' + n.body + '</div>' : ''}
    `;
    container.prepend(el);
    setTimeout(() => { el.style.opacity = '0'; el.style.transition = 'opacity 0.3s'; }, 4500);
    setTimeout(() => el.remove(), 5000);
  });
}
</script>
</body>
</html>
```
