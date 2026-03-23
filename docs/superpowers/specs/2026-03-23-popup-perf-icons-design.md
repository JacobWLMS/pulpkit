# Pulpkit: xdg_popup, Performance, and Icon Loading

## Context

Three issues need fixing together:
1. Opening/closing popups causes NIRI to rearrange windows (the bar surface changes anchors)
2. Shell CPU usage is higher than necessary from timer overhead
3. Many app icons don't load (PNG-only search, no SVG support, suboptimal size order)

## 1. xdg_popup Architecture

### Problem
The current single-surface approach expands the bar to full screen when a popup opens, then shrinks it back. This anchor change triggers compositor layout reflow. Every other Rust Wayland bar (ironbar, bar-rs, COSMIC, waybar) uses xdg_popup parented to the layer surface.

### Design
Replace the expand/shrink model with proper xdg_popup surfaces via SCTK.

**Bar surface:** Never changes. Stays anchored top, fixed height, same exclusive zone forever.

**Popup surface:** Created on demand via:
```rust
let xdg_popup = xdg_surface.get_popup(None, &positioner);
bar_layer_surface.get_popup(&xdg_popup);
```

**Positioning:** Use `XdgPositioner` to anchor the popup relative to the bar surface:
- `anchor_rect`: the button's hit-test rect on the bar (from layout)
- `gravity`: downward (popup drops below bar)
- `constraint_adjustment`: slide to stay on-screen
- For centered popups (launcher, power): anchor_rect centered on bar, gravity centered

**Popup rendering:** Each popup gets its own pixel buffer + Skia canvas. Same paint pipeline as the bar — `compute_layout` → `paint_tree` → commit. The popup's `ManagedPopup` holds a `PopupSurface` (new type wrapping xdg_popup + shm pool + buffer).

**Dismiss:** xdg_popup has a `popup_done` event — the compositor sends this when the user clicks outside or presses Escape. We handle it by setting the visibility signal to false.

**Keyboard:** The popup requests grab via `xdg_popup.grab(seat, serial)` for keyboard-interactive popups (launcher). The compositor routes keys to the grabbed popup.

### Files to change
- `pulpkit-wayland/src/surface.rs` — add `PopupSurface` type wrapping xdg_popup
- `pulpkit-wayland/src/client.rs` — handle xdg_popup events (configure, popup_done)
- `pulpkit-core/src/popups.rs` — rewrite to use PopupSurface instead of expand/shrink
- `pulpkit-core/src/surfaces.rs` — remove expand/shrink, restore simple render()
- `pulpkit-core/src/event_loop.rs` — remove expand/shrink logic, handle popup_done

### What gets deleted
- `expand()`, `shrink()` methods on ManagedSurface
- `screen_width`, `screen_height`, `expanded`, `bar_height` fields on ManagedSurface
- All anchor-change and keyboard-interactivity-change code
- The transparent-area click-dismiss logic

## 2. Performance

### Cursor blink timer
Current: fires every 530ms unconditionally, calls `cursor_blink:set(true)` even when launcher is closed.
Fix: early return in the timer callback: `if not show_launcher:get() then return end`

### Polling intervals
Current: volume/battery poll every 10s via `exec_output` (blocking subprocess).
Fix: increase to 30s. Volume changes detected via scroll interaction (instant). Battery changes are slow.

### Launcher each() resolve
Current: `filtered_apps()` iterates all ~85 apps on every layout pass.
Fix: cache filtered results in a signal, only recompute when `search_query` changes.

### Event stream overhead
Current: niri event-stream handler doesn't force re-render (correct).
No change needed.

## 3. App Icon Loading

### Search order
Current: `48x48, 64x64, 128x128, 32x32, 256x256, 24x24, 192x192, 16x16`
Fix: largest first for best downscale quality: `256x256, 192x192, 128x128, 96x96, 64x64, 48x48, 32x32, 24x24, 16x16`

### SVG support via resvg
Add `resvg` crate (pure Rust, ~2MB, no system dependencies).
Search chain: PNG at all sizes → `scalable/apps/*.svg` → pixmaps fallback.
SVG rasterization: render to 128x128 RGBA bitmap, convert to Skia Image, cache.

### Icon name variations
After exact name search fails, try:
- Strip reverse-domain prefix: `org.gnome.Nautilus` → `nautilus`
- Lowercase: `Alacritty` → `alacritty`

### Cache
Thread-local `HashMap<PathBuf, Option<Image>>` already exists. SVG-rasterized images go into the same cache. One-time cost per icon, zero cost on subsequent renders.

## Verification

1. Open/close popup — windows stay in place, no layout reflow
2. `niri msg layers` — bar stays as single surface, popup appears as separate xdg surface
3. CPU idle with bar only: < 1%
4. Launcher icons: Firefox, Ghostty, Nautilus all show real icons
5. `cargo test` passes
