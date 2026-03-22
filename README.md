# Pulpkit

A Rust + Lua desktop shell framework for Wayland.

Reactive by default, fast enough for gaming, hackable enough for daily driving.

## Features

- **Reactive engine** — SolidJS-inspired signals, computed values, and effects. Change a signal, only the affected widgets re-render.
- **Lua scripting** — Define your entire shell in Lua. Signals, widgets, timers, system commands — all from config files.
- **Wayland native** — Layer-shell surfaces via smithay-client-toolkit. Bars, popups, overlays.
- **Skia rendering** — CPU raster rendering with cached font lookups. Sub-5% idle CPU.
- **Flexbox layout** — Taffy-powered layout with Tailwind-style tokens (`bg-surface px-4 items-center gap-2`).
- **Dynamic lists** — `each()` with key-based reconciliation for workspace buttons, notification lists, etc.
- **System integration** — `exec()`, `exec_output()`, `env()` for shell commands and environment access.

## Architecture

```
pulpkit-reactive    Signal<T>, Computed<T>, Effect, batch(), Scope
    |
pulpkit-render      Skia CPU canvas, text measurement, color parsing
    |
pulpkit-wayland     Layer-shell surfaces, pointer input, output detection
    |
pulpkit-layout      Prop<T>, Node tree, Taffy flexbox, paint pipeline
    |
pulpkit-lua         Lua VM, widget constructors, signal/timer/system APIs
    |
pulpkit-core        Runtime, event loop, dirty tracking, popup state machine
```

## Quick Start

```bash
cargo build --release
./target/release/pulpkit-core examples/hello
```

## Lua API

### Widgets

```lua
row(style, children)           -- horizontal container
col(style, children)           -- vertical container
text(style, content)           -- text node (static string or function)
spacer()                       -- flex-grow: 1
button(style, opts, children)  -- interactive button with handlers
slider(style, opts)            -- draggable value slider
toggle(style, opts)            -- boolean toggle switch
each(items_fn, render_fn, key_fn)  -- dynamic list with reconciliation
```

Style is a string of Tailwind-like tokens or a reactive function:

```lua
-- Static
row("bg-base px-4 items-center gap-2", { ... })

-- Reactive (hover via signal)
local hovered = signal(false)
button(function()
  return hovered:get() and "bg-overlay px-2" or "px-2"
end, {
  on_hover = function() hovered:set(true) end,
  on_hover_lost = function() hovered:set(false) end,
}, { ... })
```

### Signals

```lua
local count = signal(0)        -- create a reactive value
count:get()                    -- read (tracks dependencies)
count:set(42)                  -- write (notifies subscribers)

local doubled = computed(function()
  return count:get() * 2       -- auto-tracks count as dependency
end)

effect(function()
  print(count:get())           -- re-runs when count changes
end)
```

### Timers

```lua
local id = set_interval(function()
  time:set(os.date("%H:%M"))
end, 1000)

local id2 = set_timeout(function()
  -- runs once after 5 seconds
end, 5000)

clear_interval(id)
clear_timeout(id2)
```

### System

```lua
exec("wpctl set-volume @DEFAULT_AUDIO_SINK@ 70%")   -- async, fire-and-forget
local vol = exec_output("wpctl get-volume @DEFAULT_AUDIO_SINK@")  -- blocking, returns stdout
local home = env("HOME")                              -- read env var
```

### Windows and Popups

```lua
window("bar", {
  monitor = "all",       -- "all", "focused", or output name
  anchor = "top",        -- "top", "bottom", "left", "right"
  exclusive = true,      -- reserve screen space
  height = 48,
}, function(ctx)
  return row("w-full h-12 bg-base", { ... })
end)

popup("volume", {
  parent = "bar",
  anchor = "top right",
  visible = show_signal,       -- Signal<bool> controls visibility
  dismiss_on_outside = true,
  width = 320,
  height = 0,                  -- 0 = auto-size from content
}, function()
  return col("bg-surface p-4", { ... })
end)
```

## Style Tokens

| Category | Tokens |
|----------|--------|
| Background | `bg-base`, `bg-surface`, `bg-overlay`, `bg-card`, `bg-<color>` |
| Text color | `text-fg`, `text-dim`, `text-muted`, `text-primary`, `text-<color>` |
| Text size | `text-xs`, `text-sm`, `text-base`, `text-lg`, `text-xl` |
| Font | `font-bold`, `font-medium` |
| Padding | `p-2`, `px-3`, `py-1` |
| Margin | `m-2`, `mx-3`, `my-1` |
| Gap | `gap-2`, `gap-4` |
| Size | `w-full`, `h-9`, `w-20`, `h-full` |
| Flex | `flex-1`, `items-center`, `justify-end`, `justify-between` |
| Rounding | `rounded`, `rounded-sm`, `rounded-lg`, `rounded-full` |
| Opacity | `opacity-50`, `opacity-80` |

## Theming

Create `theme.lua` in your shell directory:

```lua
return {
  colors = {
    base    = "#121618",
    surface = "#1a1e22",
    fg      = "#e2e6ea",
    primary = "#8cb4d8",
    -- ...
  },
  spacing_scale = 4,
  rounding = { sm = 0, md = 0, lg = 0, xl = 0, full = 0 },
  font_sizes = { xs = 10, sm = 12, base = 14, lg = 16, xl = 20 },
  font_family = "JetBrainsMono Nerd Font",
}
```

## NIRI Integration

Pulpkit detects NIRI via `NIRI_SOCKET` and can use `niri msg` for workspace switching:

```lua
local is_niri = env("NIRI_SOCKET") ~= nil

-- Poll workspaces
local raw = exec_output("niri msg -j workspaces")

-- Switch workspace
exec("niri msg action focus-workspace 3")
```

## Requirements

- Rust 2024 edition
- Wayland compositor with layer-shell support
- LuaJIT (vendored via mlua)
- Skia (vendored via skia-safe)
- Nerd Font for icons

## License

MIT
