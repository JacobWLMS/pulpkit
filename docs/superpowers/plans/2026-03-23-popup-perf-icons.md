# xdg_popup, Performance, Icons — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace expand/shrink popup model with proper xdg_popup, add SVG icon support via resvg, and optimize timer overhead.

**Architecture:** Popups become xdg_popup surfaces parented to the bar's layer surface via `layer_surface.get_popup()`. Each popup gets its own Skia buffer. Bar surface never changes anchor/size/keyboard. Icons search all sizes largest-first and rasterize SVGs via resvg.

**Tech Stack:** sctk 0.20 xdg_popup, resvg (pure Rust SVG rasterizer), existing Skia canvas

---

### Task 1: Bind XdgShell global and implement PopupHandler

**Files:**
- Modify: `crates/pulpkit-wayland/src/client.rs`
- Modify: `crates/pulpkit-wayland/Cargo.toml`

- [ ] **Step 1: Add wayland-protocols dependency**

In `Cargo.toml` (workspace root), add:
```toml
wayland-protocols = { version = "0.32", features = ["client", "unstable"] }
```

In `crates/pulpkit-wayland/Cargo.toml`, add:
```toml
wayland-protocols.workspace = true
```

- [ ] **Step 2: Add imports and XdgShell to AppState**

In `client.rs`, add to imports:
```rust
use smithay_client_toolkit::{
    delegate_xdg_popup, delegate_xdg_shell,
    shell::xdg::{XdgShell, popup::{Popup, PopupConfigure, PopupHandler}},
};
```

Add fields to `AppState`:
```rust
pub xdg_shell: XdgShell,
pub popup_done_ids: Vec<wayland_client::backend::ObjectId>,
```

- [ ] **Step 3: Initialize XdgShell in connect()**

After `LayerShell::bind()`, add:
```rust
let xdg_shell = XdgShell::bind(&globals, &qh)?;
```

Pass to AppState constructor.

- [ ] **Step 4: Implement PopupHandler for AppState**

```rust
impl PopupHandler for AppState {
    fn configure(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>,
                 popup: &Popup, config: PopupConfigure) {
        // Ack the configure
        if let Some(serial) = config.serial {
            popup.xdg_popup().ack_configure(serial);
        }
    }
    fn done(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, popup: &Popup) {
        self.popup_done_ids.push(popup.wl_surface().id());
    }
}

delegate_xdg_shell!(AppState);
delegate_xdg_popup!(AppState);
```

- [ ] **Step 5: Build and verify**

Run: `cargo check -p pulpkit-wayland`
Expected: compiles with no errors

- [ ] **Step 6: Commit**

```
git add -A && git commit -m "feat(wayland): bind XdgShell, implement PopupHandler"
```

---

### Task 2: Create PopupSurface type

**Files:**
- Modify: `crates/pulpkit-wayland/src/surface.rs`
- Modify: `crates/pulpkit-wayland/src/lib.rs`

- [ ] **Step 1: Add PopupSurface struct**

In `surface.rs`, add a new type for xdg_popup surfaces with their own buffer:

```rust
pub struct PopupSurface {
    popup: Popup,
    pool: SlotPool,
    pub width: u32,
    pub height: u32,
    buffer_data: Vec<u8>,
}
```

- [ ] **Step 2: Add PopupSurface constructor**

```rust
impl PopupSurface {
    pub fn new(
        state: &mut AppState,
        parent: &LayerSurface,
        anchor_x: i32, anchor_y: i32,
        anchor_w: i32, anchor_h: i32,
        width: u32, height: u32,
        grab: bool,
    ) -> anyhow::Result<Self> {
        // 1. Create positioner
        // 2. Configure anchor rect, size, gravity
        // 3. Create popup with NULL parent
        // 4. Reparent to layer surface via get_popup
        // 5. Optionally grab keyboard
        // 6. Commit and return
    }
}
```

Implement `get_buffer()`, `commit()`, `surface_id()`, `destroy()` methods mirroring LayerSurface.

- [ ] **Step 3: Re-export from lib.rs**

Add `pub use surface::PopupSurface;` to `lib.rs`.

- [ ] **Step 4: Build and verify**

Run: `cargo check -p pulpkit-wayland`

- [ ] **Step 5: Commit**

```
git add -A && git commit -m "feat(wayland): PopupSurface type for xdg_popup"
```

---

### Task 3: Rewrite ManagedPopup to use PopupSurface

**Files:**
- Rewrite: `crates/pulpkit-core/src/popups.rs`
- Modify: `crates/pulpkit-core/src/setup.rs`

- [ ] **Step 1: Rewrite ManagedPopup**

Replace the current expand/shrink model. ManagedPopup now holds an `Option<PopupSurface>`:

```rust
pub struct ManagedPopup {
    pub name: String,
    pub root: Node,
    pub config: PopupConfig,
    pub visible_signal: Option<Signal<DynValue>>,
    pub on_key: Option<mlua::RegistryKey>,
    pub surface: Option<PopupSurface>,  // None when hidden
    pub layout: Option<LayoutResult>,
}
```

Methods:
- `show()`: creates PopupSurface, renders content, commits
- `hide()`: drops PopupSurface (compositor destroys popup)
- `render()`: compute_layout + paint_tree + commit on the popup's own buffer
- `should_be_visible()`, `dismiss()`: same as before

- [ ] **Step 2: Update setup.rs**

Remove `x`, `y`, `backdrop` fields. Add `surface: None`.

- [ ] **Step 3: Build and verify**

Run: `cargo check -p pulpkit-core`

- [ ] **Step 4: Commit**

```
git add -A && git commit -m "feat(core): ManagedPopup uses PopupSurface"
```

---

### Task 4: Restore simple ManagedSurface (remove expand/shrink)

**Files:**
- Rewrite: `crates/pulpkit-core/src/surfaces.rs`

- [ ] **Step 1: Remove expand/shrink**

Strip `ManagedSurface` back to the original simple version:

```rust
pub struct ManagedSurface {
    pub name: String,
    pub surface: LayerSurface,
    pub root: Node,
    pub layout: Option<LayoutResult>,
    pub dirty: Rc<Cell<bool>>,
    pub hovered_node: Option<usize>,
}
```

Restore simple `render()` method (clear bg, paint_tree, commit). Remove `render_with_popups`, `expand`, `shrink`, `bar_height`, `screen_width`, `screen_height`, `expanded`.

- [ ] **Step 2: Build and verify**

Run: `cargo check -p pulpkit-core`

- [ ] **Step 3: Commit**

```
git add -A && git commit -m "refactor(core): restore simple ManagedSurface, remove expand/shrink"
```

---

### Task 5: Rewrite event loop for xdg_popup model

**Files:**
- Modify: `crates/pulpkit-core/src/event_loop.rs`

- [ ] **Step 1: Remove expand/shrink logic from the loop**

Replace the popup visibility section. Instead of expand/shrink:

```rust
// Show/hide popups via PopupSurface creation/destruction
for popup in popups.iter_mut() {
    let wants = popup.should_be_visible();
    let has_surface = popup.surface.is_some();
    if wants && !has_surface {
        popup.show(&mut client.state, bar_surface);
    } else if !wants && has_surface {
        popup.hide();
    }
}

// Handle popup_done events (compositor dismissed popup)
let done_ids: Vec<_> = client.state.popup_done_ids.drain(..).collect();
for id in done_ids {
    for popup in popups.iter_mut() {
        if popup.surface_id() == Some(&id) {
            popup.dismiss();
        }
    }
}
```

- [ ] **Step 2: Fix click dispatch for popup surfaces**

Popup clicks now arrive on the popup's own surface (separate surface_id). Restore the old click dispatch that matches surface_id to popup surfaces.

- [ ] **Step 3: Fix keyboard dispatch**

Key events on popup surfaces are identified by surface_id matching the popup's surface.

- [ ] **Step 4: Render popups independently**

```rust
for popup in popups.iter_mut() {
    if popup.surface.is_some() {
        popup.render(text_renderer, theme);
    }
}
```

- [ ] **Step 5: Build and verify**

Run: `cargo check && cargo test`

- [ ] **Step 6: Commit**

```
git add -A && git commit -m "feat(core): event loop uses xdg_popup lifecycle"
```

---

### Task 6: Add resvg for SVG icon support

**Files:**
- Modify: `Cargo.toml` (workspace)
- Modify: `crates/pulpkit-render/Cargo.toml`
- Modify: `crates/pulpkit-render/src/image.rs`

- [ ] **Step 1: Add resvg dependency**

Workspace `Cargo.toml`:
```toml
resvg = "0.44"
```

`crates/pulpkit-render/Cargo.toml`:
```toml
resvg.workspace = true
```

- [ ] **Step 2: Rewrite resolve_icon_path**

Search order: largest PNG first, then scalable SVG.

```rust
pub fn resolve_icon_path(icon_name: &str) -> Option<PathBuf> {
    // 1. Full path check
    // 2. PNG search: 256, 192, 128, 96, 64, 48, 32, 24, 16
    // 3. SVG search: scalable/apps/*.svg, *.svgz
    // 4. Pixmaps fallback
    // 5. Icon name variations (strip reverse-domain, lowercase)
}
```

- [ ] **Step 3: Add SVG rasterization to load_image**

```rust
pub fn load_image(path: &Path) -> Option<Image> {
    IMAGE_CACHE.with(|cache| {
        cache.borrow_mut()
            .entry(path.to_path_buf())
            .or_insert_with(|| {
                if path.extension().is_some_and(|e| e == "svg" || e == "svgz") {
                    load_svg(path)
                } else {
                    load_png(path)
                }
            })
            .clone()
    })
}

fn load_svg(path: &Path) -> Option<Image> {
    let tree = resvg::usvg::Tree::from_data(&std::fs::read(path).ok()?, &Default::default()).ok()?;
    let size = 128u32;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size)?;
    let scale = size as f32 / tree.size().width().max(tree.size().height());
    resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    // Convert tiny_skia pixmap (RGBA) to skia Image
    let data = skia_safe::Data::new_copy(pixmap.data());
    let info = skia_safe::ImageInfo::new((size as i32, size as i32), skia_safe::ColorType::RGBA8888, skia_safe::AlphaType::Premul, None);
    skia_safe::images::raster_from_data(&info, data, size as usize * 4)
}
```

- [ ] **Step 4: Build and verify icons load**

Run: `cargo build --release`
Test: launch bar, open launcher, check if Firefox/Ghostty show real icons.

- [ ] **Step 5: Commit**

```
git add -A && git commit -m "feat(render): SVG icon support via resvg, largest-first search"
```

---

### Task 7: Performance timer fixes

**Files:**
- Modify: `examples/hello/shell.lua`

- [ ] **Step 1: Fix cursor blink timer**

```lua
set_interval(function()
  if not show_launcher:get() then return end
  cursor_blink:set(not cursor_blink:get())
end, 530)
```

- [ ] **Step 2: Increase polling intervals**

```lua
-- Volume/battery: 30s (was 10s)
set_interval(function() ... end, 30000)
```

- [ ] **Step 3: Build, launch, measure CPU**

Run: `cargo build --release && pulpkit-core examples/hello`
Measure: `sleep 15; ps -p $PID -o %cpu=`
Expected: < 5% idle

- [ ] **Step 4: Commit**

```
git add -A && git commit -m "perf: fix cursor blink timer, reduce polling intervals"
```

---

### Task 8: Integration test and push

- [ ] **Step 1: Run all tests**

```
cargo test
```

- [ ] **Step 2: Verify popup positioning**

Open each popup (audio, network, bluetooth, battery, power, launcher, calendar).
Verify: appears below bar at correct position, dismisses on click-outside.

- [ ] **Step 3: Verify no window rearrangement**

Open and close launcher 5 times. Windows should stay in place.

- [ ] **Step 4: Verify icons**

Open launcher, check Firefox, Ghostty, foot, btop all show real icons.

- [ ] **Step 5: Final commit and push**

```
git push
```
