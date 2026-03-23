# Pulpkit Servo Shell — Design Spec

## Context

Replace Pulpkit's tiny-skia/Lua rendering stack with Servo (Rust web engine) so the shell UI is built with HTML/CSS/JS. This enables full web styling power and AI-native shell development.

## Architecture

```
Shell (HTML/CSS/JS — Preact + signals + HTM)
    │ loaded by Servo
pulpkit-servo (NEW crate)
    │ embeds Servo, SoftwareRenderingContext
    │ pushes state JSON into JS, receives commands
pulpkit-wayland (UNCHANGED)
    │ sctk layer-shell surfaces, input events, shm buffers
pulpkit-core (MODIFIED)
    │ calloop event loop + Servo waker bridge
    │ state manager, command handler
pulpkit-sub (UNCHANGED)
    │ interval, stream, exec, ipc subscriptions
```

**Dropped crates:** pulpkit-render, pulpkit-layout, pulpkit-lua

## Data Flow

1. pulpkit-sub delivers system events (niri workspace change, volume poll, etc.)
2. pulpkit-core builds state JSON object
3. pulpkit-servo pushes state into JS: `window.pulpkit.onState(json)`
4. Preact rerenders reactively from state
5. User clicks → JS calls `window.pulpkit.send({cmd, data})`
6. Servo delegate forwards command to pulpkit-core
7. Core executes system action (wpctl, niri msg, systemctl, etc.)

## Servo Integration

- **Crate:** `libservo` from git (https://github.com/servo/servo, tag v0.0.5)
- **Rendering:** `SoftwareRenderingContext` → `read_to_image()` → RGBA pixels → shm buffer
- **Input:** sctk pointer/keyboard events → `webview.notify_input_event()`
- **JS bridge:** `webview.evaluate_javascript()` to push state
- **Delegates:** `WebViewDelegate` + `ServoDelegate` for callbacks
- **Event loop:** calloop + Servo waker channel bridge (Slint pattern)

## Shell Frontend

- **Framework:** Preact + @preact/signals + HTM (no build step)
- **Files:** index.html, shell.js, shell.css, vendor/{preact,signals,htm}.mjs
- **Pattern:** State pushed from Rust → Preact signals → reactive render → commands back to Rust

## JS Bridge API

```js
// Rust → JS (state push)
window.pulpkit = {
  state: signal({}),         // reactive state from Rust
  onState(json) {            // called by Rust on state change
    this.state.value = json;
  },
  send(cmd) {                // JS → Rust command channel
    // posts to Servo delegate → Rust handler
  }
};
```

## PoC Scope (Phase 1)

Minimal proof that Servo can render HTML into a layer-shell surface:

1. sctk layer-shell surface (bar, 32px, top, exclusive)
2. Servo + SoftwareRenderingContext
3. Load simple HTML with styled text
4. read_to_image() → copy to shm buffer → display
5. Forward pointer events from sctk → Servo
6. Verify: styled HTML bar visible on Wayland compositor
