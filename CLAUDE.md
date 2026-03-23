# Pulpkit Development Guide

## Architecture

```
pulpkit-reactive    SolidJS-inspired reactivity: Signal<T>, Computed<T>, Effect, batch()
    |
pulpkit-render      Skia CPU rendering: Canvas, TextRenderer, image loading + caching
    |
pulpkit-wayland     Layer-shell surfaces, pointer/keyboard input, output tracking
    |
pulpkit-layout      Widget tree (Node enum), Taffy flexbox layout, Tailwind-style tokens
    |
pulpkit-lua         Lua VM, widget constructors, signal/timer/system/stream APIs
    |
pulpkit-core        Runtime orchestration, event loop, dirty tracking, popup state machine, IPC
```

## Key Principles

### All fixes must be universal
Every bug fix and improvement must work for ALL users, not just our specific shell config. If popup centering is broken, fix the positioning system — don't hardcode pixel offsets. If a widget has inconsistent padding, fix the widget's default style — don't override it in one shell.lua.

### Reactive by default
Every dynamic property is a `Signal<T>` or `Prop<T>`. Never use RefCell for widget state. The reactive graph tracks dependencies automatically. `Signal::set()` skips notification when value is unchanged (PartialEq).

### Framework, not shell
Every API decision asks "could a user with a different shell use this?" The Lua API should be general-purpose. Shell-specific logic lives in `examples/hello/shell.lua` and `lib.lua`, not in Rust.

### Performance is a hard requirement
Target: <1% idle CPU in release builds. The framework itself is 0.5% — any excess comes from shell.lua polling. Prefer `exec_stream()` (push-based) over `set_interval()` + `exec_output()` (pull-based).

### Small modules
No file over 300 lines. Extract when it grows.

## Crate Responsibilities

- **pulpkit-reactive**: KEEP AS-IS. Self-contained, well-tested. Only touch for bug fixes.
- **pulpkit-wayland**: KEEP AS-IS. Thin sctk wrapper. Exposes AppState, InputEvent, LayerSurface, OutputInfo.
- **pulpkit-render**: KEEP AS-IS. Skia canvas, text, image, color. Thread-local font/image caches.
- **pulpkit-layout**: Widget types (Node enum), Taffy integration, paint pipeline. Add new Node variants here.
- **pulpkit-lua**: Lua ↔ Rust bridge. Widget constructors, signal API, timer API, system API, stream API.
- **pulpkit-core**: Runtime, event loop, dirty tracking, popup state machine, IPC, setup.

## Node Types (tree.rs)

```rust
Node::Container { style, direction, children }     // row/col
Node::Text { style, content }                      // static or reactive text
Node::Image { style, path, width, height }         // PNG image from file
Node::Spacer                                       // flex-grow: 1
Node::DynamicList { style, direction, resolve, cached_children }  // each() with key reconciliation
Node::Interactive { style, kind, children }         // button/slider/toggle
```

## Lua API Surface

### Widgets
`row`, `col`, `text`, `spacer`, `image`, `button`, `slider`, `toggle`, `each`

### Signals
`signal(initial)`, `computed(fn)`, `effect(fn)`

### Timers
`set_interval(fn, ms) -> id`, `set_timeout(fn, ms) -> id`, `clear_interval(id)`, `clear_timeout(id)`

### System
`exec(cmd)`, `exec_output(cmd)`, `exec_stream(cmd, callback) -> id`, `cancel_stream(id)`, `env(name)`, `resolve_icon(name)`

### Windows
`window(name, opts, widget_fn)`, `popup(name, opts, widget_fn)`

## Popup Positioning

Popups use layer-shell surfaces with margins for positioning. All coordinates must be in LOGICAL pixels (physical / scale). Use `OutputInfo::logical_width()` / `logical_height()`.

- `anchor = "top left"` / `"top right"` — positioned below bar via top margin + left/right margin from click
- `anchor = "center"` — centered on screen using logical output dimensions
- `dismiss_on_outside = true` — creates a transparent backdrop surface that catches clicks

## Event Loop Order

```
1. calloop dispatch (waits for wayland events, IPC, stream output, timers)
2. Handle configure events (surface resize)
3. Dispatch input events (pointer motion, click, scroll, keyboard)
4. Process IPC commands (Lua eval)
5. Dispatch stream events (exec_stream callbacks)
6. Process timer cancellations
7. Fire due timers
8. Flush reactive effects
9. Check popup visibility signals (show/hide)
10. Tick popup animations
11. Mark dirty surfaces if handlers fired
12. Render dirty surfaces (single pass)
```

## Testing

```bash
cargo test              # all unit tests
cargo build --release   # release build for performance testing
```

## Style Tokens (Tailwind-like)

Spacing: `p-2`, `px-3`, `py-1`, `m-2`, `mx-3`, `gap-2`
Colors: `bg-base`, `bg-surface`, `text-fg`, `text-muted`, `text-primary`
Typography: `text-xs`, `text-sm`, `text-base`, `text-lg`, `text-xl`, `font-bold`
Layout: `flex-1`, `items-center`, `justify-end`, `justify-between`, `w-full`, `h-10`

## Common Patterns

### Adding a new widget type
1. Add variant to `Node` enum in `pulpkit-layout/src/tree.rs`
2. Handle in `build_taffy_node()` in `flex.rs` (layout)
3. Handle in `paint_tree()` in `paint.rs` (rendering)
4. Handle in `count_descendants()` in `flex.rs`
5. Handle in `wire_node()` in `pulpkit-core/src/dirty.rs` (dirty tracking)
6. Register Lua constructor in `pulpkit-lua/src/widgets.rs`

### Adding a new Lua API function
1. Add to the appropriate module in `pulpkit-lua/src/` (system.rs, timers.rs, etc.)
2. Register in the `register_*_api()` function
3. Export from `pulpkit-lua/src/lib.rs` if types need to cross crate boundaries
4. Wire in `pulpkit-core/src/runtime.rs` if it needs runtime integration
