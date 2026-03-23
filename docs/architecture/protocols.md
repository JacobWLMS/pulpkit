# Wayland Protocols

Pulpkit uses 9 Wayland protocols through its layer-shell integration (via `gtk4-layer-shell`) and the `pulpkit-wayland` crate. This page documents each protocol, its purpose, and where it appears in the codebase.

---

## Protocol Table

| Protocol | Purpose | Used by |
|----------|---------|---------|
| [wlr-layer-shell](#wlr-layer-shell) | Position surfaces as panels, popups, overlays, lock screens | All 4 surfaces |
| [ext-idle-notify](#ext-idle-notify) | Detect when the user is idle | Session idle tracking |
| [idle-inhibit](#idle-inhibit) | Prevent the compositor from marking the session idle | Caffeine mode |
| [wlr-foreign-toplevel-management](#wlr-foreign-toplevel-management) | List and manage toplevel windows | Window list, task switcher |
| [ext-session-lock](#ext-session-lock) | Lock the session and render a lock surface | Lock screen |
| [wlr-output-management](#wlr-output-management) | Query and configure display outputs | Output watcher, resolution display |
| [wlr-screencopy](#wlr-screencopy) | Capture screen contents | Screenshot commands |
| [xdg-activation](#xdg-activation) | Transfer focus to another surface | App launching |
| [virtual-keyboard](#virtual-keyboard) | Inject keyboard input | Lock screen PIN entry |
| [wlr-data-control](#wlr-data-control) | Access clipboard contents | Clipboard watcher |

---

## wlr-layer-shell

**Protocol**: `zwlr_layer_shell_v1` / `ext-layer-shell-v1`

The foundational protocol for Pulpkit. Every surface is a layer-shell surface rather than a normal XDG toplevel. This allows precise control over:

- **Layer** -- Where the surface sits in the compositor's stacking order (`Background`, `Bottom`, `Top`, `Overlay`)
- **Anchors** -- Which screen edges the surface attaches to
- **Exclusive zone** -- How much space the surface reserves (for the bar)
- **Keyboard interactivity** -- Whether the surface receives keyboard input

**Module**: `poc/src/main.rs` (via `gtk4_layer_shell::{LayerShell, Layer, Edge, KeyboardMode}`)

```
Bar:     Layer::Top,     anchored left+right+top, auto exclusive zone, keyboard=none
Popup:   Layer::Overlay, not anchored, keyboard=on-demand
Toast:   Layer::Top,     anchored top+right, keyboard=none
Lock:    Layer::Overlay, anchored all edges, keyboard=exclusive
Backdrop: Layer::Overlay, anchored all edges, keyboard=none
```

---

## ext-idle-notify

**Protocol**: `ext_idle_notification_v1`

Allows Pulpkit to be notified when the user has been idle for a configurable duration. Used in the v2 reactive architecture for driving idle-dependent behavior (dimming the bar, triggering auto-lock).

In the POC, idle state is read from logind's `IdleHint` property instead.

**Module**: `pulpkit-wayland` (v2 crate)

---

## idle-inhibit

**Protocol**: `zwp_idle_inhibit_manager_v1`

Prevents the compositor from considering the session idle. Pulpkit uses this for "caffeine mode" -- when the user toggles caffeine, an idle inhibitor is created to prevent auto-lock and screen blanking.

In the POC, caffeine state is tracked via a marker file at `$XDG_RUNTIME_DIR/pulpkit-caffeine` and the inhibitor is managed externally. The v2 architecture integrates this directly.

**Module**: `pulpkit-wayland` (v2 crate), `poc/src/watchers/caffeine.rs` (POC marker file)

---

## wlr-foreign-toplevel-management

**Protocol**: `zwlr_foreign_toplevel_manager_v1`

Provides a list of all toplevel (application) windows across all workspaces, along with their titles, app IDs, and states (activated, maximized, minimized, fullscreen). The compositor sends events when windows are created, closed, or change state.

In the POC, this information is obtained via `niri msg -j windows` and the `niri msg event-stream`. The v2 architecture can bind this protocol directly for compositor-agnostic window tracking.

**Module**: `poc/src/watchers/niri.rs` (via niri CLI), `pulpkit-wayland` (v2 crate)

---

## ext-session-lock

**Protocol**: `ext_session_lock_v1`

Locks the session and renders a lock surface on every output. The protocol guarantees that:

1. The lock surface is shown before any other content
2. No other surface can receive input while locked
3. The compositor will not render unlocked content on any output

Pulpkit's lock surface hosts a WebView with a PIN/password entry form. The `verify_password` command uses PAM to authenticate, and `unlock` calls `loginctl unlock-session` to release the lock.

**Module**: `poc/src/main.rs` (lock window setup), `poc/src/pam.rs` (PAM authentication)

---

## wlr-output-management

**Protocol**: `zwlr_output_manager_v1`

Allows querying the current output configuration (resolution, refresh rate, scale, position, enabled state) and applying changes. Pulpkit uses this to:

- Display output information in the settings popup
- Detect multi-monitor setups
- Read the logical output dimensions for popup positioning

In the POC, output information is read via `niri msg -j outputs`.

**Module**: `poc/src/watchers/outputs.rs` (via niri CLI), `pulpkit-wayland` (v2 crate)

---

## wlr-screencopy

**Protocol**: `zwlr_screencopy_manager_v1` / `ext-image-copy-capture-v1`

Captures the contents of an output or a rectangular region. Used by the screenshot commands (`screenshot`, `screenshot_full`) which are dispatched to `grim` (which uses this protocol internally).

**Module**: `poc/src/main.rs` (`handle_command` -- `grim` / `slurp`)

---

## xdg-activation

**Protocol**: `xdg_activation_v1`

Transfers activation (focus) tokens between surfaces. When Pulpkit launches an application from the app launcher, it can pass an activation token so the compositor focuses the new window immediately rather than flashing the taskbar.

**Module**: `pulpkit-wayland` (v2 crate)

---

## virtual-keyboard

**Protocol**: `zwp_virtual_keyboard_v1`

Creates a virtual keyboard that can inject key events into the compositor's input pipeline. Used by the lock screen to handle on-screen keyboard input if needed, and potentially by the v2 architecture for global keybindings.

**Module**: `pulpkit-wayland` (v2 crate)

---

## wlr-data-control

**Protocol**: `zwlr_data_control_manager_v1` / `ext-data-control-v1`

Provides access to the Wayland clipboard (selection and primary selection) without being the focused surface. Since the bar is a layer-shell surface and never receives normal keyboard focus, it cannot use the standard `wl_data_device` protocol. `wlr-data-control` allows Pulpkit to:

- Monitor clipboard changes (via `wl-paste --watch` which uses this protocol)
- Read the current clipboard contents for display in the UI

**Module**: `poc/src/watchers/clipboard.rs` (via `wl-paste --watch`)

---

## Protocol Availability

All protocols listed above are supported by **niri**, **Hyprland**, and **Sway**. Some protocols have `ext-` standardized versions that are preferred when available:

| Legacy (wlr) | Standardized (ext) | Status |
|--------------|-------------------|--------|
| `zwlr_layer_shell_v1` | `ext_layer_shell_v1` | Both widely supported |
| `zwlr_foreign_toplevel_manager_v1` | (in progress) | Use wlr version |
| `zwlr_screencopy_manager_v1` | `ext_image_copy_capture_v1` | Transition ongoing |
| `zwlr_data_control_manager_v1` | `ext_data_control_v1` | Transition ongoing |
| -- | `ext_session_lock_v1` | Standardized from the start |
| -- | `ext_idle_notification_v1` | Standardized from the start |
