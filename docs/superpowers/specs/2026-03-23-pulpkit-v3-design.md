# Pulpkit v3 Design Spec

## Problem

Pulpkit v2 has fundamental performance and architecture issues:

- **Full-surface redraws**: `damage_buffer(0, 0, w, h)` on every commit, no region tracking
- **No frame callbacks**: renders immediately regardless of compositor readiness, wastes CPU at idle
- **Monolithic event loop**: 12-parameter `run()` function mixing input, state, timers, rendering
- **Shared mutable state**: `Rc<RefCell<Vec<>>>` bags for IPC, streams, timers; leaked `Effect`s via `mem::forget`
- **Blocking Lua callbacks**: synchronous Lua execution in the event dispatch path
- **Missing HiDPI on popups**: bar surfaces honor scale, popups hardcoded to scale=1
- **No animation framework**: popup state machine exists but no interpolation or frame scheduling

These issues are structural. Incremental fixes cannot address the reactive signal graph's fundamental shared-mutation model or the lack of compositor protocol compliance (frame callbacks, damage regions).

## Solution

Rewrite the Rust internals using patterns from the COSMIC desktop environment (System76). Keep the Lua scripting layer but replace the reactive signal API with an Elm architecture (Model-View-Update), matching libcosmic's proven approach.

## Architecture

```
shell.lua (init / update / view / subscribe)
    |
    | emits Messages, returns Element trees
    v
pulpkit-core: Elm Runtime
    | message dispatch, tree diffing, frame scheduling
    v
pulpkit-render: tiny-skia (direct)
    | damage-tracked region rendering
    v
pulpkit-wayland: smithay-client-toolkit + calloop
    | layer-shell, frame callbacks, fractional-scale, HiDPI
    v
Wayland compositor
```

### Why tiny-skia Direct, Not iced_tiny_skia

COSMIC uses `iced-sctk`, a full fork of the iced runtime that replaces winit with smithay-client-toolkit. `iced_tiny_skia` and `iced_core` are not designed for standalone use outside iced's application runtime — they expect Rust `Widget` trait implementations, not Lua-generated trees. Since pulpkit's widgets come from Lua tables, we need our own widget tree, diffing, and layout — making iced coupling unnecessary.

We use `tiny-skia` directly as the 2D rasterizer (the same library COSMIC's iced fork uses under the hood). The existing v2 canvas API (`draw_rounded_rect`, `draw_text`, `draw_image`, `clip_rect`, `translate`) ports from `skia-safe` to `tiny-skia` with minimal changes (~174 lines).

### Data Flow

1. **Event arrives** (input, timer, stream, IPC) via calloop source
2. **Message created** — typed `RuntimeMsg` enum variant
3. **Lua `update(state, msg)`** called — returns state table
4. **Lua `view(state)`** called — returns widget Element tree (always, after every update)
5. **Widget diff** — custom tree diff compares old and new Lua element trees (see Widget Diffing)
6. **Layout pass** — taffy relayouts changed branches
7. **Damage calculation** — changed widgets produce damage rectangles
8. **Frame callback gate** — rendering waits for `wl_surface.frame()` callback
9. **Render** — tiny-skia paints only damaged regions into shm buffer
10. **Commit** — `wl_surface.damage_buffer(x, y, w, h)` per region, not full surface

### State Change Detection

`view()` is called after every `update()` call, unconditionally. The tree diff (step 5) handles short-circuiting — if the view produces an identical tree, no layout/render work happens. This avoids the Lua table identity problem (update mutates in-place, so identity comparison is useless) and keeps the runtime simple.

### Idle Behavior

At true idle (no events, no active subscriptions producing events):
- calloop blocks on epoll with no timeout
- No Lua execution
- No rendering
- Near-zero CPU usage (frame callbacks prevent spurious wakeups)

Note: subscriptions like `stream("pactl subscribe", ...)` will wake the loop when PulseAudio emits events system-wide, triggering Lua update + view + diff. True 0.0% CPU requires no active event-producing subscriptions.

### Adaptive Frame Scheduling

Stolen from cosmic-panel's tiered approach:
- **Animating**: 16ms calloop timeout (~60fps)
- **Interactive** (hover, drag): render on next frame callback after state change
- **Idle**: calloop blocks indefinitely until next event
- **Hidden surfaces**: 300ms minimum dispatch interval

## Crate Structure

```
pulpkit/
  crates/
    pulpkit-core/      Elm runtime, message dispatch, ShellState, frame scheduling
    pulpkit-render/    tiny-skia canvas, text rendering (fontdb+rustybuzz), damage tracker
    pulpkit-wayland/   sctk layer-shell, frame callbacks, fractional-scale-v1, input, output
    pulpkit-lua/       Lua VM, view/update/subscribe bridge, widget constructors, msg() API
    pulpkit-layout/    taffy integration, Element tree, widget diffing, style tokens
    pulpkit-sub/       Subscription system: timers, exec_stream, exec, dbus, config watch, IPC
```

### Dropped Crates

- **pulpkit-reactive** — signals, computed, effects, batch all replaced by Elm state management
- **pulpkit-plugin** — functionality folded into pulpkit-lua

### New Crate: pulpkit-sub

Owns all external event sources. Each subscription is a calloop event source that produces typed messages through a channel. Replaces the current `Rc<RefCell<Vec<>>>` pattern for IPC commands, stream events, and timer tracking.

Subscription types:
- `interval(ms, msg_name)` — calloop `Timer` source
- `timeout(ms, msg_name)` — one-shot calloop `Timer`, auto-removed after firing
- `stream(cmd, msg_name)` — spawns subprocess, reads stdout via calloop channel
- `exec(cmd, msg_name)` — one-shot command execution, result delivered as message payload
- `dbus(bus, interface, signal, msg_name)` — zbus subscription bridged to calloop
- `config_watch(path, msg_name)` — inotify via calloop
- `ipc(msg_name)` — Unix socket listener

## Widget Diffing

Since widgets are Lua tables (not Rust trait objects), we implement our own diffing algorithm in `pulpkit-layout`.

### Diff Algorithm

Each Element node has: `type` (widget kind), `key` (optional, for lists), `props` (style, text, event handlers), `children`.

Comparison rules:
1. **Type mismatch** → full replace (destroy old subtree, create new)
2. **Type match, no key** → compare by position in parent's child list
3. **Type match, with key** → match by key (used inside `each()` lists)
4. **Props comparison** → shallow compare each prop field; changed props mark the node as damaged
5. **Children diff** → recursive; for keyed lists, use LCS-style key matching to minimize re-creation

### Damage Tracking

When a node is marked damaged (props changed or replaced):
- Its bounding box (from the previous layout pass) becomes a damage rectangle
- If the new node has different layout properties (size, padding, flex), re-layout the subtree and add the new bounding box as additional damage
- Damage rectangles are merged (union of overlapping rects) before rendering

### Keyed List Diffing (each)

`each(list, key_field, render_fn)` produces keyed children. The differ matches old and new lists by key:
- **Same key, same position** → diff the subtree
- **Same key, moved** → reuse the subtree, update position (damage both old and new locations)
- **Key removed** → destroy subtree, damage old location
- **Key added** → create subtree, damage new location

This matches v2's DynamicList reconciliation but operates on immutable snapshots rather than signal closures.

## Hover State

Hover is the most frequent interaction in a shell bar (fires on every pointer motion). Routing all hover through Lua update/view would be expensive. Instead, hover is handled at two levels:

### Rust-Level Hover (default)

CSS-style hover states (`hover:bg-surface`, `hover:text-primary`) are resolved in Rust during the paint phase, not in Lua. The renderer checks whether the pointer is within each node's bounding box and applies hover style variants automatically. This requires no Lua execution and no state change — it's a paint-time decision, identical to how CSS `:hover` works in browsers.

The pointer motion event updates a `hovered_node: Option<NodeId>` field in ShellState. The paint pass reads this to decide which hover styles to apply. Only the old and new hovered node bounding boxes become damage regions.

### Lua-Level Hover (opt-in)

For cases where hover needs to trigger logic (e.g., showing a tooltip, starting a delayed action), widgets accept `on_hover = msg("name")` and `on_hover_lost = msg("name")`. These fire messages through the normal update cycle. Shell authors use this sparingly — most hover is purely visual and handled by Rust.

```lua
-- Pure visual hover (no Lua involvement, handled in Rust):
button({ style = "hover:bg-surface", on_click = msg("open") }, icon("power"))

-- Logic hover (goes through Lua update):
button({ on_hover = msg("show_tooltip", "Power"),
         on_hover_lost = msg("hide_tooltip"),
         on_click = msg("open") }, icon("power"))
```

## Lua API

### Lifecycle Functions

```lua
-- Called once at startup. Returns initial state table.
function init()
  return {
    vol = 75,
    brightness = 50,
    popup_open = false,
    workspaces = {},
    user = nil,  -- loaded async via exec in subscribe
    host = nil,
  }
end

-- Called for every message. Returns state table.
-- Optionally returns a second value: a Task for async work.
function update(state, msg)
  if msg.type == "vol_up" then
    state.vol = math.min(state.vol + 5, 100)
  elseif msg.type == "vol_down" then
    state.vol = math.max(state.vol - 5, 0)
  elseif msg.type == "toggle_popup" then
    state.popup_open = not state.popup_open
  elseif msg.type == "workspaces" then
    state.workspaces = msg.data
  elseif msg.type == "user_info" then
    state.user = msg.data.user
    state.host = msg.data.host
  end
  return state
end

-- Called after every update(). Returns widget tree.
-- Must be a pure function of state — no side effects, no exec_output().
function view(state)
  return window("bar", { anchor = "top", height = 40, exclusive = true },
    row({ style = "bg-base h-full items-center px-2 gap-2" },
      -- workspace indicators
      each(state.workspaces, "id", function(ws)
        return button({ on_click = msg("switch_ws", ws.id),
                         style = ws.active and "bg-primary" or "bg-surface" })
      end),
      spacer(),
      -- volume button
      button({ on_click = msg("toggle_popup"),
               style = "rounded-full p-2 hover:bg-surface" },
        icon(state.vol > 50 and "vol-high" or "vol-low")
      )
    )
  )
end

-- Called after every update(). Returns list of active subscriptions.
-- Runtime diffs subscriptions: starts new ones, stops removed ones.
function subscribe(state)
  return {
    stream("pactl subscribe", "audio_event"),
    interval(60000, "check_battery"),
    exec("hyprctl workspaces -j", "workspaces"),
  }
end
```

### Widget Constructors

Same widget vocabulary as v2, but all return Element values instead of Node trees with embedded signals:

- `row(opts, ...)` / `col(opts, ...)` — flex containers
- `text(string)` / `text(opts, string)` — static text
- `icon(name)` — Nerd Font glyph or resolved icon path
- `image(path, opts)` — image file
- `button(opts, ...)` — clickable container, `on_click` takes a `msg()` value
- `slider(opts)` — `value`, `min`, `max`, `on_change = msg("name")`
- `toggle(opts)` — `checked`, `on_toggle = msg("name")`
- `input(opts)` — text input, `value`, `on_input = msg("name")`
- `each(list, key_fn, render_fn)` — keyed list rendering
- `spacer()` — flex-grow filler
- `scroll(opts, ...)` — scrollable container

### Message API

```lua
msg("name")              -- simple message: { type = "name" }
msg("name", data)        -- message with payload: { type = "name", data = data }
```

Messages are inert values — they don't execute anything. The runtime calls `update()` when they're triggered by user interaction or subscriptions.

### Subscription API

```lua
interval(ms, msg_name)              -- periodic timer
timeout(ms, msg_name)               -- one-shot timer (auto-removed after firing)
stream(cmd, msg_name)               -- subprocess stdout, line-by-line
exec(cmd, msg_name)                 -- one-shot command, result as msg data
dbus(bus, path, iface, signal, msg) -- D-Bus signal subscription
config_watch(path, msg_name)        -- file change watcher
```

Subscriptions are declarative — the runtime manages their lifecycle. If `subscribe()` stops returning a subscription, the runtime tears it down (cancels timer, kills process, etc.).

### Window and Popup API

```lua
-- In view(), return one or more surfaces:
function view(state)
  local surfaces = {}

  -- Bar surface (one per monitor when monitor = "all")
  table.insert(surfaces, window("bar", {
    anchor = "top", height = 40, exclusive = true, monitor = "all"
  }, bar_content(state)))

  -- Popup (only present when state says so)
  if state.popup_open then
    table.insert(surfaces, popup("vol", {
      anchor = "top right", width = 300, height = 200,
      dismiss_on_outside = true,
    }, popup_content(state)))
  end

  return surfaces
end
```

Popups are separate top-level surfaces in the view return value, not nested inside windows. This matches the Wayland model (popups are independent layer-shell or xdg-popup surfaces). The runtime diffs the surface list: new surfaces are created, removed surfaces are destroyed.

Multi-monitor: `monitor = "all"` causes the runtime to clone the window onto every output. `monitor = "primary"` or a specific output name targets one display.

### Error Handling

If `update()` or `view()` throws a Lua error:
- The error is logged via `log::error!`
- The previous widget tree and state are kept (no crash, no blank screen)
- A `msg("lua_error", error_string)` is queued so the shell can optionally display it
- The runtime continues processing events normally

This prevents a typo in shell.lua from killing the entire desktop shell.

## Theme System

The v2 theme system carries over. A `theme.lua` file in the shell directory defines colors, font family, and font sizes. Tailwind-style tokens (`bg-base`, `text-primary`, `bg-surface`, etc.) resolve against theme values at style parse time.

```lua
-- theme.lua (unchanged from v2)
return {
  font_family = "JetBrainsMono Nerd Font",
  font_size = 14,
  colors = {
    base    = "#1e2128",
    surface = "#282c34",
    primary = "#8cb4d8",
    fg      = "#c8ccd4",
    muted   = "#8a929a",
  },
}
```

Theme is loaded once at startup and passed to the renderer. Hot-reloading theme changes can be added later via a `config_watch()` subscription.

## Rendering

### Backend: tiny-skia (direct)

Using `tiny-skia` as a direct 2D rasterizer, not through iced. This gives us:
- Pure Rust (no C dependencies unlike skia-safe)
- Faster compilation (~5s vs ~60s for skia-safe)
- Sufficient API for shell UI (paths, rounded rects, text via external shaper, images)
- Full control over damage tracking and buffer management

### Text Rendering

Since tiny-skia has no built-in text support (unlike skia-safe), we need a text stack:
- `fontdb` — font discovery and loading
- `rustybuzz` — OpenType shaping (for Nerd Font glyphs and complex scripts)
- `ab_glyph` — glyph rasterization to bitmaps
- Glyph cache: `HashMap<(GlyphId, FontSize), tiny_skia::Pixmap>` — rasterize each glyph once

This replaces skia-safe's integrated font manager. The glyph cache keeps text rendering fast after the first frame.

### Damage Tracking

Two levels:
1. **Widget-level**: the tree diff identifies changed nodes. Each changed node's bounding box becomes a damage region.
2. **Surface-level**: `wl_surface.damage_buffer()` called with per-region rectangles, not full surface.

When rendering a damaged region:
- Save canvas state, clip to damage rect
- Paint the background of every ancestor that overlaps the damage rect
- Paint the damaged node and its descendants
- Restore canvas state

Multiple damage rects are merged when they overlap or are close together (within 8px) to avoid excessive clip/restore cycles.

### Buffer Management

- Single shm `SlotPool` per surface (same as v2)
- tiny-skia renders into a `tiny_skia::PixmapMut` wrapping the shm buffer directly (no intermediate `Vec<u8>` copy)
- Buffer scale derived from `fractional-scale-v1` or `wl_output.scale`

### Frame Callbacks

After every `wl_surface.commit()`, register `wl_surface.frame()`. The callback delivers a `FrameCallback(SurfaceId)` message to the calloop event loop. No rendering occurs until this callback fires.

This is the single most important performance change — it eliminates all CPU usage at idle.

## Wayland Integration

### Protocol Support

| Protocol | Purpose | v2 Status | v3 Status |
|---|---|---|---|
| wlr-layer-shell | Bar/panel surfaces | Implemented | Keep |
| xdg-popup | Dropdown popups | Implemented | Keep |
| wl_shm | Shared memory buffers | Implemented | Keep |
| wl_surface.frame | Frame callbacks | Missing | Add |
| fractional-scale-v1 | HiDPI scaling | Missing | Add |
| wp-viewporter | Buffer scaling | Missing | Add |
| cursor-shape-v1 | Cursor without surface | Missing | Nice-to-have |

### HiDPI

All surfaces (bar and popups) will:
1. Bind `fractional-scale-v1` if available, fall back to `wl_output.scale`
2. Set buffer scale and allocate physical-pixel buffers accordingly
3. Use `wp-viewporter` for precise fractional scaling

## Event Loop

### calloop Source Registration

```
EventLoop<ShellState>
  ├── WaylandSource (sctk event queue)
  ├── Channel<RuntimeMsg> (internal message bus)
  ├── Timer sources (from subscriptions)
  ├── Channel<(SubId, String)> (stream/exec subprocess output)
  ├── Generic<UnixListener> (IPC socket)
  └── Frame callback wakeups (via channel)
```

### ShellState

```rust
struct ShellState {
    lua: Lua,
    app_state: LuaRegistryKey,       // the Lua state table in registry
    prev_tree: Option<ElementTree>,   // previous view() result for diffing
    surfaces: Vec<ManagedSurface>,
    subscriptions: SubscriptionManager,
    damage: Vec<DamageRect>,          // pending damage regions per surface
    frame_ready: HashSet<SurfaceId>,  // surfaces with pending frame callbacks
    hovered_node: Option<NodeId>,     // for Rust-level hover styles
    msg_queue: Vec<RuntimeMsg>,
    theme: Theme,
}
```

### Loop Iteration

```
1. calloop.dispatch(timeout, &mut state)
2. Drain msg_queue into batch
3. For each msg in batch:
   a. Call Lua update(state, msg) -> state
4. Call Lua view(state) -> new_tree
5. Diff new_tree against prev_tree:
   a. For each changed node: compute damage rect from layout position
   b. For each added/removed node: damage the affected region
6. Call Lua subscribe(state) -> new_subs
7. Diff new_subs against active_subs: start/stop subscriptions
8. For each dirty surface with frame_ready:
   a. Clip to damage regions
   b. Render damaged widgets (tiny-skia)
   c. damage_buffer() per region
   d. Commit
   e. Register next frame callback
   f. Clear frame_ready for this surface
9. Store new_tree as prev_tree
```

### Message Batching

Multiple messages that arrive in the same calloop dispatch (e.g., several stream lines at once) are batched — `update()` is called once per message, but `view()` is called only once after all messages are processed. This prevents redundant tree diffs.

## Performance Targets

| Metric | v2 Current | v3 Target |
|---|---|---|
| Idle CPU (no active subs) | 0.5% (timers) | 0.0% (frame callbacks) |
| Idle CPU (with stream sub) | 0.5% | <0.1% (update+diff, no render) |
| Slider drag CPU | ~5% (full redraw) | <1% (region redraw) |
| Memory (RSS) | ~45MB (Skia) | 25-30MB (tiny-skia + fontdb) |
| Startup time | ~200ms | ~150ms (no Skia C++ init) |
| Compile time | ~90s (skia-safe) | ~30s (pure Rust) |

## Migration Path

This is a breaking change to the Lua API. The migration:

1. `signal()` / `computed()` / `effect()` -> state fields in `init()` + logic in `update()`
2. `set_interval()` / `exec_stream()` -> `subscribe()` returning `interval()` / `stream()`
3. `set_timeout()` -> `subscribe()` returning `timeout()`
4. `on_click = function() ... end` -> `on_click = msg("name")`
5. Reactive `Prop<T>` styles -> plain style strings in `view()`
6. `popup()` visibility via signals -> conditional inclusion in `view()` surfaces based on state
7. Hover signals (`signal(false)` + `on_hover/on_hover_lost`) -> `hover:` CSS-style tokens for visual, `on_hover = msg()` for logic
8. `exec_output()` in view functions -> `exec()` subscription in `subscribe()`, result stored in state

### Migration Scope

The existing `shell.lua` (~736 lines) and `lib.lua` (~362 lines) will need a full rewrite. Key transformations:
- ~30 hover signals become `hover:` style tokens (no Lua state needed)
- `exec_output("whoami")` / `exec_output("hostname")` in popup builders become `exec()` subscriptions that populate state fields
- `each()` keyed lists change from signal closures to plain data from state
- Derived data (e.g., filtered app lists) moves from `computed()` to helper functions called in `view()`

## Dependencies

### Added
- `tiny-skia` — 2D rasterizer (pure Rust)
- `fontdb` — font discovery
- `rustybuzz` — OpenType text shaping
- `ab_glyph` — glyph rasterization
- `zbus` (optional) — D-Bus subscriptions

### Removed
- `skia-safe` — replaced by tiny-skia (pure Rust, faster compile)
- `resvg` — kept only if SVG icon theme support is needed; can be made optional feature

### Kept
- `smithay-client-toolkit 0.20` — Wayland client
- `calloop 0.14` — event loop
- `mlua` + LuaJIT — Lua scripting
- `wayland-client 0.31` — protocol bindings
- `taffy 0.9` — flexbox layout
- `anyhow`, `log` — error handling, logging

### Removed (unused)
- `tokio` — not used in v2, not needed in v3 (calloop is the event loop)

## Out of Scope

- GPU rendering (wgpu backend) — future enhancement, tiny-skia is sufficient for shell UI
- Hot reload of Lua — deferred from v2, still deferred
- Process isolation for plugins — COSMIC does this but adds significant complexity
- Animation framework — frame scheduling supports it, but animation API is a separate design
- `malloc::trim()` optimization — worth adding later for long-running process memory, but not in initial v3
