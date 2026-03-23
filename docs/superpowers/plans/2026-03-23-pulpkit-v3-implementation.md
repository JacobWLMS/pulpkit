# Pulpkit v3 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rewrite Pulpkit internals with Elm architecture, tiny-skia rendering, damage tracking, and frame callbacks for a resource-efficient Wayland shell.

**Architecture:** Elm MVU (init/update/view/subscribe in Lua) with custom widget tree diffing, tiny-skia CPU rendering with region-based damage tracking, calloop event loop with typed message channels, and Wayland frame callback gating for zero idle CPU.

**Tech Stack:** Rust (2024 edition), tiny-skia, fontdb + rustybuzz + ab_glyph (text), smithay-client-toolkit 0.20, calloop 0.14, mlua + LuaJIT, taffy 0.9

**Spec:** `docs/superpowers/specs/2026-03-23-pulpkit-v3-design.md`

---

## File Structure

```
pulpkit/
  Cargo.toml                           # workspace: 6 crates (drop reactive, plugin)
  crates/
    pulpkit-render/
      Cargo.toml                       # tiny-skia, fontdb, rustybuzz, ab_glyph
      src/
        lib.rs                         # pub mod + re-exports
        color.rs                       # Color type (port from v2, drop skia dep)
        canvas.rs                      # tiny-skia canvas (port draw_* from v2)
        text.rs                        # fontdb + rustybuzz + ab_glyph text stack
        image.rs                       # PNG loading via image crate, icon resolve
    pulpkit-layout/
      Cargo.toml                       # taffy, pulpkit-render
      src/
        lib.rs                         # pub mod + re-exports
        element.rs                     # Element enum (replaces Node), ElementTree
        style.rs                       # StyleProps + parse (port from v2, add hover:)
        theme.rs                       # Theme (port from v2)
        flex.rs                        # taffy layout (port from v2, adapt to Element)
        diff.rs                        # NEW: tree diffing algorithm
        damage.rs                      # NEW: damage rect tracking + merging
        paint.rs                       # paint pipeline (port from v2, add damage clip)
    pulpkit-wayland/
      Cargo.toml                       # sctk, calloop, wayland-client
      src/
        lib.rs                         # pub mod + re-exports
        client.rs                      # AppState + WaylandClient (port, add frame cb)
        surface.rs                     # LayerSurface (port, add frame callbacks)
        input.rs                       # InputEvent enum (port from v2)
        output.rs                      # OutputInfo (port from v2)
    pulpkit-lua/
      Cargo.toml                       # mlua, pulpkit-layout
      src/
        lib.rs                         # pub mod + re-exports
        vm.rs                          # LuaVm (port from v2)
        element.rs                     # NEW: Lua table -> Element conversion
        widgets.rs                     # widget constructors (row, col, text, etc.)
        msg.rs                         # NEW: msg() API, Message type
        subscribe.rs                   # NEW: subscription descriptors from Lua
        bridge.rs                      # NEW: init/update/view/subscribe Lua calls
    pulpkit-sub/
      Cargo.toml                       # calloop, NEW crate
      src/
        lib.rs                         # pub mod + re-exports
        manager.rs                     # SubscriptionManager: diff, start, stop
        interval.rs                    # calloop Timer subscription
        timeout.rs                     # one-shot Timer subscription
        stream.rs                      # exec_stream subprocess subscription
        exec.rs                        # one-shot exec subscription
        ipc.rs                         # Unix socket IPC (port from v2 core/ipc.rs)
    pulpkit-core/
      Cargo.toml                       # all crates + calloop + mlua
      src/
        lib.rs                         # pub mod + pub fn run()
        runtime.rs                     # ShellState, setup, Lua loading
        event_loop.rs                  # main loop (clean, <150 lines)
        hover.rs                       # NEW: Rust-level hover state + damage
        surfaces.rs                    # ManagedSurface (port + simplify)
        main.rs                        # Binary entry point: calls lib::run()
```

---

## Phase 1: Foundation Crates (no Wayland, fully testable)

### Task 1: Scaffold v3 Workspace

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Delete contents of: all `crates/*/src/` files
- Create: new `Cargo.toml` for each crate

This task guts the workspace, removes dropped crates (pulpkit-reactive, pulpkit-plugin), adds the new crate (pulpkit-sub), and updates all dependencies. The old source is preserved in git history.

- [ ] **Step 1: Create a v3 branch**

```bash
cd ~/pulpkit
git checkout -b v3
```

- [ ] **Step 2: Remove dropped crates and old source**

Delete `crates/pulpkit-reactive/` and `crates/pulpkit-plugin/` directories entirely. Clear the `src/` contents of all remaining crates (we'll rebuild them task by task).

```bash
rm -rf crates/pulpkit-reactive crates/pulpkit-plugin
# Clear source files but keep directories
for crate in pulpkit-render pulpkit-layout pulpkit-wayland pulpkit-lua pulpkit-core; do
  rm -f crates/$crate/src/*.rs
  echo "// v3 placeholder" > crates/$crate/src/lib.rs
done
```

- [ ] **Step 3: Create pulpkit-sub crate**

```bash
mkdir -p crates/pulpkit-sub/src
echo "// v3 placeholder" > crates/pulpkit-sub/src/lib.rs
```

- [ ] **Step 4: Write root Cargo.toml**

```toml
[workspace]
resolver = "2"
members = [
    "crates/pulpkit-render",
    "crates/pulpkit-layout",
    "crates/pulpkit-wayland",
    "crates/pulpkit-lua",
    "crates/pulpkit-sub",
    "crates/pulpkit-core",
]

[workspace.package]
version = "0.3.0"
edition = "2024"
license = "MIT"

[workspace.dependencies]
pulpkit-render = { path = "crates/pulpkit-render" }
pulpkit-layout = { path = "crates/pulpkit-layout" }
pulpkit-wayland = { path = "crates/pulpkit-wayland" }
pulpkit-lua = { path = "crates/pulpkit-lua" }
pulpkit-sub = { path = "crates/pulpkit-sub" }
pulpkit-core = { path = "crates/pulpkit-core" }

# Rendering
tiny-skia = "0.11"
fontdb = "0.22"
rustybuzz = "0.20"
ab_glyph = "0.2"
image = { version = "0.25", default-features = false, features = ["png"] }

# Wayland
smithay-client-toolkit = { version = "0.20", features = ["calloop"] }
wayland-client = "0.31"
wayland-cursor = "0.31"
calloop = "0.14"

# Scripting
mlua = { version = "0.11", features = ["luajit", "vendored"] }

# Layout
taffy = "0.9"

# Utilities
anyhow = "1"
log = "0.4"
env_logger = "0.11"
```

- [ ] **Step 5: Write each crate's Cargo.toml**

`crates/pulpkit-render/Cargo.toml`:
```toml
[package]
name = "pulpkit-render"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
tiny-skia.workspace = true
fontdb.workspace = true
rustybuzz.workspace = true
ab_glyph.workspace = true
image.workspace = true
anyhow.workspace = true
log.workspace = true
```

`crates/pulpkit-layout/Cargo.toml`:
```toml
[package]
name = "pulpkit-layout"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
pulpkit-render.workspace = true
taffy.workspace = true
anyhow.workspace = true
log.workspace = true
```

`crates/pulpkit-wayland/Cargo.toml`:
```toml
[package]
name = "pulpkit-wayland"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
smithay-client-toolkit.workspace = true
wayland-client.workspace = true
wayland-cursor.workspace = true
calloop.workspace = true
log.workspace = true
anyhow.workspace = true
```

`crates/pulpkit-lua/Cargo.toml`:
```toml
[package]
name = "pulpkit-lua"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
pulpkit-layout.workspace = true
pulpkit-render.workspace = true
mlua.workspace = true
anyhow.workspace = true
log.workspace = true
```

`crates/pulpkit-sub/Cargo.toml`:
```toml
[package]
name = "pulpkit-sub"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
calloop.workspace = true
anyhow.workspace = true
log.workspace = true
```

`crates/pulpkit-core/Cargo.toml`:
```toml
[package]
name = "pulpkit-core"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
pulpkit-render.workspace = true
pulpkit-layout.workspace = true
pulpkit-wayland.workspace = true
pulpkit-lua.workspace = true
pulpkit-sub.workspace = true
calloop.workspace = true
wayland-client.workspace = true
anyhow.workspace = true
mlua.workspace = true
log.workspace = true
env_logger.workspace = true
```

- [ ] **Step 6: Verify workspace compiles**

Run: `cargo check`
Expected: compiles with no errors (all libs are empty placeholders)

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "scaffold: v3 workspace with 6 crates, drop reactive+plugin"
```

---

### Task 2: pulpkit-render — Color + Canvas

**Files:**
- Create: `crates/pulpkit-render/src/color.rs`
- Create: `crates/pulpkit-render/src/canvas.rs`
- Modify: `crates/pulpkit-render/src/lib.rs`

Port the Color type (drop skia dependency) and Canvas (rewrite from skia-safe to tiny-skia). Canvas API: `clear`, `draw_rounded_rect`, `draw_image`, `clip_rect`, `save`, `restore`, `translate`, `fill_rect`. Text rendering is a separate task.

Reference: v2 `crates/pulpkit-render/src/color.rs` (78 lines), `crates/pulpkit-render/src/canvas.rs` (174 lines) — accessible in git history on the `master` branch.

- [ ] **Step 1: Write Color type with tests**

Port from v2's color.rs. `Color` struct with `r, g, b, a` fields, `from_hex()`, `to_premultiplied_argb_u32()`, `Default`. No skia dependency.

Test: `from_hex("#ff0000")` returns Color { r: 255, g: 0, b: 0, a: 255 }. Test: `from_hex("#80ff0000")` returns Color with alpha 128. Test: `Default` is transparent black.

- [ ] **Step 2: Run tests, verify pass**

Run: `cargo test -p pulpkit-render`

- [ ] **Step 3: Write Canvas wrapping tiny_skia::PixmapMut**

The v2 Canvas wraps a `skia_safe::Surface`. The v3 Canvas wraps a `tiny_skia::PixmapMut<'a>` (borrowed from shm buffer). API:

```rust
pub struct Canvas<'a> {
    pixmap: tiny_skia::PixmapMut<'a>,
}

impl<'a> Canvas<'a> {
    pub fn from_buffer(data: &'a mut [u8], width: u32, height: u32) -> Option<Self>;
    pub fn clear(&mut self, color: Color);
    pub fn fill_rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: Color);
    pub fn draw_rounded_rect(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32, color: Color);
    pub fn draw_image(&mut self, x: f32, y: f32, w: f32, h: f32, image: &tiny_skia::Pixmap);
    pub fn clip_rect(&mut self, x: f32, y: f32, w: f32, h: f32);
    pub fn save(&mut self);
    pub fn restore(&mut self);
    pub fn translate(&mut self, dx: f32, dy: f32);
}
```

For `draw_rounded_rect`: use `tiny_skia::PathBuilder` to build a rounded rect path, then `pixmap.fill_path()`. For clip/save/restore: maintain a manual clip stack (tiny-skia has `ClipMask` but no built-in save/restore — track transforms and clips in a `Vec<CanvasState>`).

Test: create a 100x100 buffer, `clear(white)`, verify pixel at (0,0) is white. Test: `fill_rect` at (10,10,20,20) with red, verify pixel at (15,15) is red.

- [ ] **Step 4: Run tests, verify pass**

Run: `cargo test -p pulpkit-render`

- [ ] **Step 5: Update lib.rs with exports**

```rust
pub mod color;
pub mod canvas;
pub use color::Color;
pub use canvas::Canvas;
```

- [ ] **Step 6: Commit**

```bash
git add crates/pulpkit-render/
git commit -m "feat(render): Color type + tiny-skia Canvas with rounded rects"
```

---

### Task 3: pulpkit-render — Text Rendering

**Files:**
- Create: `crates/pulpkit-render/src/text.rs`
- Modify: `crates/pulpkit-render/src/canvas.rs` (add `draw_text`)
- Modify: `crates/pulpkit-render/src/lib.rs`

Build the text rendering stack: fontdb for font loading, rustybuzz for shaping, ab_glyph for glyph rasterization, and a glyph cache.

- [ ] **Step 1: Write TextRenderer struct**

```rust
pub struct TextRenderer {
    db: fontdb::Database,
    glyph_cache: RefCell<HashMap<GlyphCacheKey, tiny_skia::Pixmap>>,
}

struct GlyphCacheKey {
    font_id: fontdb::ID,
    glyph_id: u16,
    size_tenths: u32,  // font_size * 10, for cache key stability
}
```

Methods:
- `TextRenderer::new()` — creates fontdb::Database, loads system fonts
- `TextRenderer::measure_text(text, family, size) -> (f32, f32)` — returns (width, height)
- `TextRenderer::draw_text(canvas, text, x, y, family, size, color)` — shapes, rasterizes, composites

For shaping: look up font in fontdb, load font data, create `rustybuzz::Face`, shape the text with `rustybuzz::shape()`, iterate glyph positions. For rasterization: use `ab_glyph::Font::outline_glyph()` then rasterize into a `tiny_skia::Pixmap`. Cache by `GlyphCacheKey`.

- [ ] **Step 2: Write tests**

Test: `measure_text("Hello", "sans-serif", 14.0)` returns a width > 0 and height > 0.
Test: `draw_text` on a 200x40 canvas at (0, 0) doesn't panic.
Test: glyph cache hit — calling `draw_text` twice with same params should use cache (verify cache len == num unique glyphs, not 2x).

- [ ] **Step 3: Run tests, verify pass**

Run: `cargo test -p pulpkit-render`

- [ ] **Step 4: Add draw_text to Canvas**

Add `Canvas::draw_text(&mut self, text: &str, x: f32, y: f32, size: f32, family: &str, color: Color, renderer: &TextRenderer)`.

This delegates to `TextRenderer::draw_text` which composites glyphs onto the canvas's pixmap.

- [ ] **Step 5: Commit**

```bash
git add crates/pulpkit-render/
git commit -m "feat(render): text rendering with fontdb + rustybuzz + glyph cache"
```

---

### Task 4: pulpkit-render — Image Loading

**Files:**
- Create: `crates/pulpkit-render/src/image.rs`
- Modify: `crates/pulpkit-render/src/lib.rs`

Port image loading from v2. Load PNGs via the `image` crate, convert to `tiny_skia::Pixmap`. Thread-local cache. Icon path resolution.

- [ ] **Step 1: Write image loading with cache**

```rust
thread_local! {
    static IMAGE_CACHE: RefCell<HashMap<String, Option<tiny_skia::Pixmap>>> = RefCell::new(HashMap::new());
}

pub fn load_image(path: &Path) -> Option<tiny_skia::Pixmap>;
pub fn resolve_icon_path(name: &str) -> Option<String>;
```

`load_image`: check cache, if miss load via `image::open()`, convert to RGBA8, create `tiny_skia::Pixmap::from_vec()`.

`resolve_icon_path`: same logic as v2 — check XDG icon directories for the name.

- [ ] **Step 2: Write tests**

Test: `load_image` with a nonexistent path returns None.
Test: `load_image` called twice with same path uses cache (second call is faster — or just verify cache contains the key).

- [ ] **Step 3: Commit**

```bash
git add crates/pulpkit-render/
git commit -m "feat(render): PNG image loading with thread-local cache"
```

---

### Task 5: pulpkit-layout — Element Tree + StyleProps

**Files:**
- Create: `crates/pulpkit-layout/src/element.rs`
- Create: `crates/pulpkit-layout/src/style.rs`
- Create: `crates/pulpkit-layout/src/theme.rs`
- Modify: `crates/pulpkit-layout/src/lib.rs`

Define the v3 Element tree (replaces v2's Node+Prop+Signal). Elements are plain data — no reactive signals, no closures. StyleProps is ported from v2 with the `hover:` token addition.

- [ ] **Step 1: Define Element enum**

```rust
// element.rs
pub type NodeId = usize;

#[derive(Debug, Clone)]
pub enum Element {
    Container {
        style: StyleProps,
        hover_style: Option<StyleProps>,  // parsed from "hover:bg-surface" tokens
        direction: Direction,
        children: Vec<Element>,
    },
    Text {
        style: StyleProps,
        content: String,
    },
    Image {
        style: StyleProps,
        path: String,
        width: f32,
        height: f32,
    },
    Spacer,
    Button {
        style: StyleProps,
        hover_style: Option<StyleProps>,
        on_click: Option<Message>,
        on_hover: Option<Message>,
        on_hover_lost: Option<Message>,
        children: Vec<Element>,
    },
    Slider {
        style: StyleProps,
        value: f64,
        min: f64,
        max: f64,
        on_change: Option<Message>,
    },
    Toggle {
        style: StyleProps,
        checked: bool,
        on_toggle: Option<Message>,
    },
    Input {
        style: StyleProps,
        value: String,
        placeholder: String,
        on_input: Option<Message>,
    },
    Scroll {
        style: StyleProps,
        children: Vec<Element>,
    },
    Each {
        style: StyleProps,
        direction: Direction,
        children: Vec<KeyedChild>,
    },
}

#[derive(Debug, Clone)]
pub struct KeyedChild {
    pub key: String,
    pub element: Element,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub msg_type: String,
    pub data: Option<MessageData>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MessageData {
    String(String),
    Float(f64),
    Bool(bool),
    Int(i64),
    Table(Vec<(String, MessageData)>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Direction { Row, Column }
```

Key difference from v2: no `Prop<T>`, no `Signal`, no `Rc<dyn Fn()>`. All data is owned values. `Message` is an inert data type with `PartialEq`.

- [ ] **Step 2: Port StyleProps + parse with hover support**

Port from v2's `style.rs` (352 lines). Add `hover:` prefix support — when `parse()` encounters `hover:bg-surface`, it returns the base style in `StyleProps` and the hover overrides in a separate `Option<StyleProps>`.

```rust
pub fn parse_with_hover(tokens: &str, theme: &Theme) -> (StyleProps, Option<StyleProps>);
```

Split tokens by space. Any token starting with `hover:` goes into the hover set. Parse base tokens → base StyleProps. Parse hover tokens (strip `hover:` prefix) → overlay StyleProps. If no hover tokens, return None.

- [ ] **Step 3: Port Theme from v2**

Port `theme.rs` — `Theme` struct with `font_family`, `font_size`, `colors` HashMap. `Theme::load_from_lua()` and `Theme::default_slate()`.

- [ ] **Step 4: Write tests**

Test: Element::Container with 2 children, verify children.len() == 2.
Test: `parse_with_hover("bg-base hover:bg-surface p-2", theme)` returns base with bg_color=base and hover with bg_color=surface.
Test: Message equality: `Message { msg_type: "click".into(), data: None } == Message { msg_type: "click".into(), data: None }`.
Test: `Theme::default_slate()` has expected color keys.

- [ ] **Step 5: Run tests, verify pass**

Run: `cargo test -p pulpkit-layout`

- [ ] **Step 6: Commit**

```bash
git add crates/pulpkit-layout/
git commit -m "feat(layout): Element tree, StyleProps with hover:, Theme, Message type"
```

---

### Task 6: pulpkit-layout — Tree Diffing

**Files:**
- Create: `crates/pulpkit-layout/src/diff.rs`
- Modify: `crates/pulpkit-layout/src/lib.rs`

The tree differ compares two Element trees and produces a list of changes. This is the core of the Elm render optimization.

- [ ] **Step 1: Define diff output types**

```rust
// diff.rs
#[derive(Debug, PartialEq)]
pub enum DiffResult {
    /// Trees are identical — no changes needed
    Same,
    /// Trees differ — here are the changes
    Changed(Vec<DiffChange>),
}

#[derive(Debug, PartialEq)]
pub enum DiffChange {
    /// Node at this path was replaced with a different type
    Replace { path: Vec<usize> },
    /// Node's props changed (same type)
    PropsChanged { path: Vec<usize> },
    /// Child added at index
    ChildAdded { path: Vec<usize>, index: usize },
    /// Child removed at index
    ChildRemoved { path: Vec<usize>, index: usize },
    /// Keyed child moved from old_index to new_index
    ChildMoved { path: Vec<usize>, old_index: usize, new_index: usize },
}
```

- [ ] **Step 2: Implement diff_trees()**

```rust
pub fn diff_trees(old: &[Element], new: &[Element]) -> DiffResult;
```

Algorithm (recursive):
1. Compare element types (discriminant). Mismatch → `Replace`.
2. Same type → compare props (style, text content, value, checked, etc. — compare by value, not identity). Changed → `PropsChanged`.
3. For children: if neither is keyed (`Each`), diff by position. If keyed, diff by key matching.
4. Return `Same` if no changes found in entire tree.

Keyed diff: build a HashMap of old keys → index. Walk new list: if key exists in old, recursively diff that child. If not, `ChildAdded`. Walk old keys not in new: `ChildRemoved`. Track index changes: `ChildMoved`.

- [ ] **Step 3: Write tests**

Test: identical trees → `Same`.
Test: text content changed → `PropsChanged { path: [0] }`.
Test: child added → `ChildAdded`.
Test: child removed → `ChildRemoved`.
Test: different element type at same position → `Replace`.
Test: keyed children reordered → `ChildMoved`.
Test: deeply nested change → path has multiple indices.

- [ ] **Step 4: Run tests, verify pass**

Run: `cargo test -p pulpkit-layout -- diff`

- [ ] **Step 5: Commit**

```bash
git add crates/pulpkit-layout/src/diff.rs crates/pulpkit-layout/src/lib.rs
git commit -m "feat(layout): tree diffing algorithm with keyed list support"
```

---

### Task 7: pulpkit-layout — Damage Tracking

**Files:**
- Create: `crates/pulpkit-layout/src/damage.rs`
- Modify: `crates/pulpkit-layout/src/lib.rs`

Damage tracker converts diff changes + layout positions into damage rectangles.

- [ ] **Step 1: Define DamageRect and DamageTracker**

```rust
// damage.rs
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DamageRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl DamageRect {
    pub fn union(self, other: DamageRect) -> DamageRect;
    pub fn overlaps(&self, other: &DamageRect) -> bool;
    pub fn expand(&self, margin: i32) -> DamageRect;
}

/// Merge overlapping or close damage rects to reduce clip/restore cycles.
pub fn merge_damage(rects: Vec<DamageRect>, merge_distance: i32) -> Vec<DamageRect>;
```

- [ ] **Step 2: Write tests**

Test: two overlapping rects merge into their union.
Test: two distant rects stay separate.
Test: rects within `merge_distance` (8px) merge.
Test: `DamageRect::union` produces correct bounding box.

- [ ] **Step 3: Run tests, verify pass**

Run: `cargo test -p pulpkit-layout -- damage`

- [ ] **Step 4: Commit**

```bash
git add crates/pulpkit-layout/src/damage.rs crates/pulpkit-layout/src/lib.rs
git commit -m "feat(layout): damage rect tracking and merging"
```

---

### Task 8: pulpkit-layout — Taffy Layout + Paint

**Files:**
- Create: `crates/pulpkit-layout/src/flex.rs`
- Create: `crates/pulpkit-layout/src/paint.rs`
- Modify: `crates/pulpkit-layout/src/lib.rs`

Port the taffy layout engine and paint pipeline from v2, adapted for the new Element type. Paint now accepts damage rects and clips rendering.

- [ ] **Step 1: Port flex.rs layout engine**

Port from v2 `flex.rs` (412 lines). Replace `Node` with `Element`, remove `Prop::resolve()` calls (all values are direct), remove `DynamicList::resolve` closure call (use `Each::children` directly).

Key types:
```rust
pub struct LayoutResult {
    pub nodes: Vec<LayoutNode>,
}
pub struct LayoutNode {
    pub x: f32, pub y: f32,
    pub width: f32, pub height: f32,
    pub element_index: usize,  // index into flat element list for diff correlation
}
pub fn compute_layout(root: &Element, width: f32, height: f32, text: &TextRenderer, font: &str) -> LayoutResult;
pub fn hit_test(layout: &LayoutResult, x: f32, y: f32) -> Option<usize>;
```

- [ ] **Step 2: Port paint.rs with damage clipping**

Port from v2 `paint.rs` (255 lines). Add damage-aware painting:

```rust
pub fn paint_tree(
    canvas: &mut Canvas,
    layout: &LayoutResult,
    elements: &[Element],  // flat element list matching layout nodes
    font_family: &str,
    text_renderer: &TextRenderer,
    damage: Option<&[DamageRect]>,  // if Some, only paint within these rects
    hovered_node: Option<usize>,    // for Rust-level hover styles
);
```

When `damage` is `Some`: for each damage rect, `save()` + `clip_rect()` + paint all nodes overlapping the rect + `restore()`. When `None` (first frame): paint everything.

- [ ] **Step 3: Write tests**

Test: `compute_layout` with a row containing text + spacer + text, verify positions.
Test: `hit_test` finds correct node index.
Test: `paint_tree` with full damage (None) doesn't panic on a simple tree.

- [ ] **Step 4: Run tests, verify pass**

Run: `cargo test -p pulpkit-layout`

- [ ] **Step 5: Commit**

```bash
git add crates/pulpkit-layout/
git commit -m "feat(layout): taffy layout engine + damage-clipped paint pipeline"
```

---

## Phase 2: Lua Bridge + Subscriptions

### Task 9: pulpkit-lua — VM + Message API

**Files:**
- Create: `crates/pulpkit-lua/src/vm.rs`
- Create: `crates/pulpkit-lua/src/msg.rs`
- Modify: `crates/pulpkit-lua/src/lib.rs`

Port the LuaVm from v2 and implement the `msg()` API.

- [ ] **Step 1: Port LuaVm**

Same as v2 — creates Lua state with LuaJIT, `load_file()`, `lua()` accessor.

- [ ] **Step 2: Implement msg() Lua global**

```rust
// msg.rs
pub fn register_msg_api(lua: &Lua) -> mlua::Result<()>;
```

Registers global `msg(name, data?)` that returns a Lua table `{ type = name, data = data }`. The table is tagged with a metatable `__pulpkit_msg = true` so the runtime can distinguish messages from regular tables.

Test: `msg("click")` returns table with type="click", data=nil.
Test: `msg("set_vol", 75)` returns table with type="set_vol", data=75.
Test: `msg("scroll", { delta = -1 })` returns table with data being the sub-table.

- [ ] **Step 3: Run tests, verify pass**

Run: `cargo test -p pulpkit-lua`

- [ ] **Step 4: Commit**

```bash
git add crates/pulpkit-lua/
git commit -m "feat(lua): LuaVm port + msg() API for Elm messages"
```

---

### Task 10: pulpkit-lua — Widget Constructors

**Files:**
- Create: `crates/pulpkit-lua/src/widgets.rs`
- Create: `crates/pulpkit-lua/src/element.rs`
- Modify: `crates/pulpkit-lua/src/lib.rs`

Register Lua globals for all widget constructors. Each returns a Lua userdata wrapping an `Element`.

- [ ] **Step 1: Define LuaElement userdata**

```rust
// element.rs
pub struct LuaElement(pub Element);
impl mlua::UserData for LuaElement {}

/// Convert a Lua value (LuaElement userdata or table) to Element
pub fn lua_to_element(val: mlua::Value) -> mlua::Result<Element>;

/// Convert a Lua table to Message
pub fn lua_to_message(val: mlua::Value) -> mlua::Result<Option<Message>>;
```

- [ ] **Step 2: Register widget constructors**

```rust
// widgets.rs
pub fn register_widgets(lua: &Lua, theme: Arc<Theme>) -> mlua::Result<()>;
```

Registers: `row(opts, ...)`, `col(opts, ...)`, `text(string)` / `text(opts, string)`, `icon(name)`, `image(path, opts)`, `button(opts, ...)`, `slider(opts)`, `toggle(opts)`, `input(opts)`, `each(list, key, render_fn)`, `spacer()`, `scroll(opts, ...)`, `window(name, opts, child)`, `popup(name, opts, child)`.

`window()` and `popup()` return tagged Lua tables (metatable `__pulpkit_surface = true`) containing `name`, `kind` ("window"/"popup"), `opts`, and the root `LuaElement`. The Elm bridge reads these in `view()` conversion.

Each constructor parses style tokens via `StyleProps::parse_with_hover()`, extracts message values from `on_click`/`on_change` etc., builds an `Element`, wraps in `LuaElement`.

The `opts` parameter is always a Lua table. Style is extracted from `opts.style` string. Children are varargs after opts.

- [ ] **Step 3: Write tests**

Test: `row({ style = "bg-base p-2" }, text("hello"))` produces Container with 1 child.
Test: `button({ on_click = msg("open"), style = "hover:bg-surface" }, text("Go"))` produces Button with hover_style set and on_click = Message.
Test: `slider({ value = 50, min = 0, max = 100, on_change = msg("vol") })` produces Slider with correct fields.
Test: `each({...}, "id", fn)` with a list of 3 items produces Each with 3 KeyedChildren.

- [ ] **Step 4: Run tests, verify pass**

Run: `cargo test -p pulpkit-lua`

- [ ] **Step 5: Commit**

```bash
git add crates/pulpkit-lua/
git commit -m "feat(lua): widget constructors for v3 Element tree"
```

---

### Task 11: pulpkit-lua — Elm Bridge (init/update/view/subscribe)

**Files:**
- Create: `crates/pulpkit-lua/src/bridge.rs`
- Create: `crates/pulpkit-lua/src/subscribe.rs`
- Modify: `crates/pulpkit-lua/src/lib.rs`

The bridge calls the four Elm lifecycle functions in Lua and converts results to Rust types.

- [ ] **Step 1: Define SubscriptionDef type**

```rust
// subscribe.rs
#[derive(Debug, Clone, PartialEq)]
pub enum SubscriptionDef {
    Interval { ms: u64, msg_name: String },
    Timeout { ms: u64, msg_name: String },
    Stream { cmd: String, msg_name: String },
    Exec { cmd: String, msg_name: String },
    ConfigWatch { path: String, msg_name: String },
    Dbus { bus: String, path: String, iface: String, signal: String, msg_name: String },
    Ipc { msg_name: String },
}

pub fn parse_subscriptions(lua: &Lua, table: mlua::Table) -> mlua::Result<Vec<SubscriptionDef>>;
```

Also register Lua globals `interval(ms, msg)`, `timeout(ms, msg)`, `stream(cmd, msg)`, `exec(cmd, msg)`, `config_watch(path, msg)`, `dbus(bus, path, iface, signal, msg)`, `ipc(msg)` that return tagged tables.

Subscription matching semantics: two subscriptions are "the same" if they have the same variant and same `msg_name`. If other params change (e.g., interval ms), the old subscription is stopped and a new one started.

- [ ] **Step 2: Implement ElmBridge**

```rust
// bridge.rs
pub struct ElmBridge {
    init_key: mlua::RegistryKey,
    update_key: mlua::RegistryKey,
    view_key: mlua::RegistryKey,
    subscribe_key: mlua::RegistryKey,
    state_key: mlua::RegistryKey,
}

impl ElmBridge {
    /// Load shell.lua into Lua, extract init/update/view/subscribe functions.
    pub fn load(lua: &Lua, shell_path: &Path) -> mlua::Result<Self>;

    /// Call init(), store returned state table.
    pub fn init(&self, lua: &Lua) -> mlua::Result<()>;

    /// Call update(state, msg). Msg is a Lua table with {type, data}.
    pub fn update(&self, lua: &Lua, msg: &Message) -> mlua::Result<()>;

    /// Call view(state), convert returned tree to Vec<SurfaceDef>.
    pub fn view(&self, lua: &Lua, theme: &Theme) -> mlua::Result<Vec<SurfaceDef>>;

    /// Call subscribe(state), parse returned subscriptions.
    pub fn subscribe(&self, lua: &Lua) -> mlua::Result<Vec<SubscriptionDef>>;
}

pub struct SurfaceDef {
    pub name: String,
    pub kind: SurfaceKind,       // Window or Popup
    pub anchor: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub exclusive: bool,
    pub monitor: MonitorTarget,
    pub dismiss_on_outside: bool,
    pub root: Element,
}

pub enum SurfaceKind { Window, Popup }
pub enum MonitorTarget { All, Primary, Named(String) }
```

`view()` conversion: Lua's `view(state)` returns a list of `window()`/`popup()` calls. The `window(name, opts, child)` and `popup(name, opts, child)` globals must be registered in Task 10's `register_widgets()` — they return tagged Lua tables with `__pulpkit_surface = true` metatable, containing name, opts, and root element. The bridge iterates the returned list and converts each to a `SurfaceDef`.

Note: `window()` and `popup()` are registered alongside widget constructors in Task 10, not here.

- [ ] **Step 3: Write tests**

Test with a minimal shell Lua string:
```lua
function init() return { count = 0 } end
function update(state, msg)
  if msg.type == "inc" then state.count = state.count + 1 end
  return state
end
function view(state)
  return { window("test", { anchor = "top", height = 40 },
    text(tostring(state.count))
  ) }
end
function subscribe(state) return { interval(1000, "tick") } end
```

Test: `bridge.init()` succeeds. Test: `bridge.update(msg{type="inc"})` succeeds. Test: `bridge.view()` returns 1 SurfaceDef with name="test". Test: `bridge.subscribe()` returns 1 SubscriptionDef::Interval.

- [ ] **Step 4: Run tests, verify pass**

Run: `cargo test -p pulpkit-lua`

- [ ] **Step 5: Commit**

```bash
git add crates/pulpkit-lua/
git commit -m "feat(lua): Elm bridge — init/update/view/subscribe lifecycle"
```

---

### Task 12: pulpkit-sub — Manager + Timer Subscriptions

**Files:**
- Create: `crates/pulpkit-sub/src/manager.rs`
- Create: `crates/pulpkit-sub/src/interval.rs`
- Create: `crates/pulpkit-sub/src/timeout.rs`
- Modify: `crates/pulpkit-sub/src/lib.rs`

The subscription manager and simple timer-based subscriptions. The manager diffs subscription lists and starts/stops subscriptions.

- [ ] **Step 1: Define SubscriptionManager and SubMessage**

```rust
// manager.rs
use calloop::channel::Sender;

pub struct SubscriptionManager {
    active: Vec<ActiveSub>,
    msg_sender: Sender<SubMessage>,
}

pub struct SubMessage {
    pub msg_type: String,
    pub data: Option<String>,
}

struct ActiveSub {
    def: SubscriptionDef,
    handle: SubHandle,
}

enum SubHandle {
    Timer(calloop::RegistrationToken),
    Process(std::process::Child, calloop::RegistrationToken),
    Channel(calloop::RegistrationToken),
}
```

Matching semantics: two `SubscriptionDef`s are "the same" when they have the same discriminant and same `msg_name`. If other params differ (e.g., interval ms), the old sub is stopped and a new one started. This is implemented via a `sub_key()` method returning `(discriminant, msg_name)`.

```rust
impl SubscriptionManager {
    pub fn new(sender: Sender<SubMessage>) -> Self;
    pub fn reconcile(&mut self, new_subs: Vec<SubscriptionDef>, handle: &calloop::LoopHandle<'_, S>);
}
```

- [ ] **Step 2: Implement interval and timeout**

`interval.rs`: `calloop::timer::Timer` with `TimeoutAction::ToDuration` for repeating.
`timeout.rs`: same but `TimeoutAction::Drop` after firing once.

- [ ] **Step 3: Write tests**

Test: `reconcile([interval(1000, "tick")])` starts a timer (active count == 1).
Test: `reconcile([])` stops the timer (active count == 0).
Test: `reconcile` with identical list is a no-op.
Test: `reconcile([interval(2000, "tick")])` after `interval(1000, "tick")` restarts (same msg_name, different ms).
Test: `reconcile([interval(1000, "tick"), timeout(5000, "alert")])` starts both.

- [ ] **Step 4: Run tests, verify pass**

Run: `cargo test -p pulpkit-sub`

- [ ] **Step 5: Commit**

```bash
git add crates/pulpkit-sub/
git commit -m "feat(sub): subscription manager + interval/timeout subscriptions"
```

---

### Task 13: pulpkit-sub — Stream, Exec, IPC, Config Watch

**Files:**
- Create: `crates/pulpkit-sub/src/stream.rs`
- Create: `crates/pulpkit-sub/src/exec.rs`
- Create: `crates/pulpkit-sub/src/ipc.rs`
- Create: `crates/pulpkit-sub/src/config_watch.rs`
- Modify: `crates/pulpkit-sub/src/manager.rs` (add SubHandle variants)
- Modify: `crates/pulpkit-sub/src/lib.rs`

Process-based and file-based subscriptions.

Note: D-Bus subscription (`dbus.rs` + `zbus` dependency) is deferred to a follow-up task. The `Dbus` variant exists in `SubscriptionDef` but `reconcile()` logs a warning and skips it until implemented. This keeps the initial v3 scope manageable.

- [ ] **Step 1: Implement stream subscription**

Spawns `sh -c <cmd>` as a subprocess, reads stdout line-by-line in a background thread, sends lines through a `calloop::channel`. Port from v2 `runtime.rs:181-216`. The `SubHandle::Process` stores the `Child` for cleanup.

- [ ] **Step 2: Implement exec (one-shot)**

Spawns command in a background thread, reads full stdout, sends result as a single `SubMessage`, then the manager auto-removes it from the active list on next reconcile.

- [ ] **Step 3: Port IPC socket**

Port from v2 `pulpkit-core/src/ipc.rs`. Unix domain socket at `$XDG_RUNTIME_DIR/pulpkit.sock`. When `Ipc { msg_name }` is in the subscription list, the manager starts the listener. Incoming commands are sent as `SubMessage { msg_type: msg_name, data: Some(command_text) }`.

- [ ] **Step 4: Implement config_watch**

Uses `calloop::generic::Generic` wrapping an inotify fd (or the `notify` crate with calloop bridging). Watches a file path, sends a message when the file changes.

- [ ] **Step 5: Write tests**

Test: stream subscription starts a subprocess and receives at least one line (use `echo hello` as cmd).
Test: exec subscription returns full stdout of `echo hello` as a single message.
Test: config_watch with a temp file — write to file, verify message received.

- [ ] **Step 6: Run tests, verify pass**

Run: `cargo test -p pulpkit-sub`

- [ ] **Step 7: Commit**

```bash
git add crates/pulpkit-sub/
git commit -m "feat(sub): stream, exec, IPC, config_watch subscriptions"
```

---

## Phase 3: Wayland + Runtime Integration

### Task 14: pulpkit-wayland — Port with Frame Callbacks

**Files:**
- Create: `crates/pulpkit-wayland/src/client.rs`
- Create: `crates/pulpkit-wayland/src/surface.rs`
- Create: `crates/pulpkit-wayland/src/input.rs`
- Create: `crates/pulpkit-wayland/src/output.rs`
- Modify: `crates/pulpkit-wayland/src/lib.rs`

Port from v2 with two critical additions: frame callbacks and fractional-scale-v1.

- [ ] **Step 1: Port client.rs**

Port `AppState` and `WaylandClient` from v2 (678 lines). Changes:
- Add `frame_callbacks: HashSet<ObjectId>` to AppState — tracks which surfaces have pending frame callbacks
- Implement `CompositorHandler::frame()` — when the compositor sends a frame callback, add the surface's ObjectId to `frame_callbacks` set
- Remove: popup_done_ids, popup_configured_ids (simplified popup model in v3)

- [ ] **Step 2: Port surface.rs with frame callback support**

Port `LayerSurface` from v2 (612 lines). Changes:
- Replace `buffer_data: Vec<u8>` with direct `SlotPool` buffer management (no intermediate copy)
- Add `request_frame(&self)` method: calls `self.layer.wl_surface().frame(&qh, self.layer.wl_surface().clone())`
- Add `commit_with_damage(&mut self, damage: &[DamageRect])` that calls `damage_buffer()` per rect instead of full surface
- Apply `fractional-scale-v1` if available via sctk 0.20's `FractionalScalingManager`
- Note on buffer access: the `SlotPool::create_buffer()` returns a `(WlBuffer, &mut [u8])`. Wrap the `&mut [u8]` slice in `tiny_skia::PixmapMut::from_bytes()` for direct rendering. This avoids the v2 intermediate `Vec<u8>` copy.
- `wp-viewporter` and `cursor-shape-v1` are deferred to a follow-up task

- [ ] **Step 3: Port input.rs and output.rs**

Straight port from v2. InputEvent enum, OutputInfo struct.

- [ ] **Step 4: Verify compilation**

Run: `cargo check -p pulpkit-wayland`
Expected: compiles (cannot run tests without a Wayland display)

- [ ] **Step 5: Commit**

```bash
git add crates/pulpkit-wayland/
git commit -m "feat(wayland): sctk port with frame callbacks + damage regions"
```

---

### Task 15: pulpkit-core — Elm Runtime + Event Loop

**Files:**
- Create: `crates/pulpkit-core/src/runtime.rs`
- Create: `crates/pulpkit-core/src/event_loop.rs`
- Create: `crates/pulpkit-core/src/hover.rs`
- Create: `crates/pulpkit-core/src/surfaces.rs`
- Create: `crates/pulpkit-core/src/main.rs` (binary entry point)
- Modify: `crates/pulpkit-core/src/lib.rs`

The main runtime: sets up everything, runs the event loop. `lib.rs` exports `pub fn run()`, `main.rs` calls it.

- [ ] **Step 1: Define ShellState and RuntimeMsg**

```rust
// runtime.rs
pub enum RuntimeMsg {
    Input(InputEvent),
    Subscription(SubMessage),
    FrameCallback(ObjectId),
    Ipc(String),
}

pub struct ShellState {
    pub lua: Lua,
    pub bridge: ElmBridge,
    pub theme: Arc<Theme>,
    pub text_renderer: TextRenderer,
    pub surfaces: Vec<ManagedSurface>,
    pub sub_manager: SubscriptionManager,
    pub prev_surfaces: Vec<SurfaceDef>,
    pub msg_queue: Vec<RuntimeMsg>,
    pub frame_ready: HashSet<ObjectId>,
    pub hovered_node: Option<usize>,
}
```

- [ ] **Step 2: Write setup function**

```rust
pub fn run(shell_dir: PathBuf) -> anyhow::Result<()>;
```

Initialization sequence:
1. Create Lua VM
2. Load theme.lua
3. Register widget constructors, msg API, subscription globals
4. Load shell.lua via ElmBridge
5. Call `bridge.init()`
6. Connect to Wayland
7. Call `bridge.view()` to get initial surface list
8. Create ManagedSurfaces for each SurfaceDef
9. Call `bridge.subscribe()` to get initial subscriptions
10. Set up calloop sources (Wayland, msg channel, sub channel, IPC)
11. Initial render (full, no damage)
12. Enter event loop

- [ ] **Step 3: Write event loop**

```rust
// event_loop.rs — target: <150 lines
pub fn run_loop(
    client: &mut WaylandClient,
    state: &mut ShellState,
) -> anyhow::Result<()>;
```

Loop body (per spec):
1. `calloop.dispatch(timeout, &mut client.state)`
2. Collect input events → RuntimeMsg::Input
3. Drain msg_queue batch
4. For each msg: call `bridge.update(lua, msg)`
5. Call `bridge.view(lua, theme)` → new surface defs
6. Diff surface list: create/destroy surfaces
7. For each surface: diff element tree, compute damage
8. Call `bridge.subscribe(lua)` → reconcile subscriptions
9. Handle hover (pointer position vs layout — see hover.rs)
10. For each dirty surface with frame_ready: render, commit, request_frame

Error handling: wrap `bridge.update()` and `bridge.view()` in `match`. On Lua error: log, keep previous tree, queue `lua_error` message.

Multi-monitor: when a `SurfaceDef` has `monitor = MonitorTarget::All`, create one `ManagedSurface` per connected output. When outputs change, create/destroy surfaces to match.

- [ ] **Step 4: Write hover.rs**

```rust
// hover.rs
pub fn update_hover(
    state: &mut ShellState,
    pointer_x: f64,
    pointer_y: f64,
    surface_id: &ObjectId,
) -> Vec<DamageRect>;
```

Hit-test the layout to find which node the pointer is over. If changed from `hovered_node`, return damage rects for old and new node bounding boxes. The paint pass uses `hovered_node` to apply `hover_style` overrides.

- [ ] **Step 5: Write surfaces.rs**

```rust
// surfaces.rs
pub struct ManagedSurface {
    pub def: SurfaceDef,
    pub surface: LayerSurface,
    pub layout: Option<LayoutResult>,
    pub prev_elements: Vec<Element>,  // for diffing
    pub dirty: bool,
}
```

Methods for creating from SurfaceDef, rendering (layout + paint + commit).

- [ ] **Step 6: Verify compilation**

Run: `cargo check -p pulpkit-core`

- [ ] **Step 7: Commit**

```bash
git add crates/pulpkit-core/
git commit -m "feat(core): Elm runtime with event loop, hover, surface management"
```

---

### Task 16: End-to-End Integration Test

**Files:**
- Create: `examples/minimal/shell.lua`
- Create: `examples/minimal/theme.lua`
- Modify: `crates/pulpkit-core/src/lib.rs` (add binary entry point)

Write a minimal shell.lua that exercises the full pipeline.

- [ ] **Step 1: Write minimal shell.lua**

```lua
function init()
  return { count = 0, time = "..." }
end

function update(state, msg)
  if msg.type == "tick" then
    state.count = state.count + 1
    state.time = os.date("%H:%M:%S")
  end
  return state
end

function view(state)
  return {
    window("bar", { anchor = "top", height = 36, exclusive = true, monitor = "all" },
      row({ style = "bg-base h-full items-center px-3 gap-2" },
        text({ style = "text-sm text-primary font-bold" }, "pulpkit v3"),
        spacer(),
        text({ style = "text-xs text-muted" }, state.time),
        text({ style = "text-xs text-fg" }, "ticks: " .. state.count)
      )
    )
  }
end

function subscribe(state)
  return {
    interval(1000, "tick"),
  }
end
```

- [ ] **Step 2: Write minimal theme.lua**

```lua
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

- [ ] **Step 3: Write Lua error recovery test**

Unit test (does not need Wayland): create an ElmBridge with a shell.lua whose `view()` throws an error. Verify that calling `bridge.view()` returns `Err`, and that calling it again after fixing the error returns `Ok` with the correct tree. This validates that a Lua error doesn't corrupt the bridge state.

```rust
#[test]
fn lua_error_in_view_recovers() {
    // Setup bridge with: view() that errors on state.crash == true
    // 1. Call view() with crash=false -> Ok
    // 2. Set crash=true via update(), call view() -> Err
    // 3. Set crash=false via update(), call view() -> Ok (recovered)
}
```

- [ ] **Step 4: Build and run on Wayland**

```bash
cargo build --release 2>&1
# Run on Wayland session:
./target/release/pulpkit-core examples/minimal/
```

Expected: a bar appears at the top of the screen with "pulpkit v3", time, and a tick counter updating every second. CPU at idle (between ticks) should be 0.0%.

- [ ] **Step 4: Verify frame callbacks work**

Check with `top` or `htop`: between tick updates, CPU usage should be 0.0% (not 0.5% like v2). The bar should only repaint once per second when the tick counter changes.

- [ ] **Step 5: Commit**

```bash
git add examples/ crates/pulpkit-core/src/lib.rs
git commit -m "feat: end-to-end v3 working — minimal shell with Elm lifecycle"
```

---

### Task 17: Interactive Widgets (button, slider, toggle, popup)

**Files:**
- Modify: `crates/pulpkit-core/src/event_loop.rs`
- Modify: `crates/pulpkit-core/src/hover.rs`
- Create: `examples/interactive/shell.lua`

Add click dispatch, slider drag, toggle, and popup support to the event loop.

- [ ] **Step 1: Add click dispatch**

In the event loop, on `PointerButton` press: hit-test the layout, find the clicked element. If it's a `Button` with `on_click`, queue the message. If it's a `Toggle`, queue the `on_toggle` message with `!current_checked`. If it's a `Slider`, begin drag tracking.

- [ ] **Step 2: Add slider drag**

Port the slider drag state from v2 event_loop.rs (lines 36-45). On `PointerMotion` during drag: compute new value from pointer position, queue `on_change` message. On mouse-up: end drag.

- [ ] **Step 3: Add popup surface management**

When `bridge.view()` returns a new popup SurfaceDef that wasn't in the previous list: create a new LayerSurface (overlay layer). When a popup disappears from the list: destroy the surface.

For `dismiss_on_outside`: when a click lands on the bar (not on a popup), and there are visible popups with `dismiss_on_outside = true`, queue a dismiss message. The shell.lua `update()` handles this by setting `state.popup_open = false`, which causes `view()` to omit the popup, which causes the runtime to destroy the surface.

- [ ] **Step 4: Write interactive example**

```lua
function init()
  return { vol = 50, dark = true, popup = false }
end

function update(state, msg)
  if msg.type == "toggle_popup" then state.popup = not state.popup
  elseif msg.type == "set_vol" then state.vol = msg.data
  elseif msg.type == "toggle_dark" then state.dark = not state.dark
  elseif msg.type == "dismiss" then state.popup = false
  end
  return state
end

function view(state)
  local surfaces = {
    window("bar", { anchor = "top", height = 36, exclusive = true },
      row({ style = "bg-base h-full items-center px-3 gap-2" },
        button({ on_click = msg("toggle_popup"), style = "p-2 hover:bg-surface rounded" },
          text({ style = "text-sm" }, "Vol: " .. math.floor(state.vol))
        ),
        spacer(),
        toggle({ checked = state.dark, on_toggle = msg("toggle_dark") })
      )
    )
  }
  if state.popup then
    table.insert(surfaces, popup("vol", {
      anchor = "top left", width = 200, height = 80,
      dismiss_on_outside = true,
    },
      col({ style = "bg-surface p-4 rounded-lg gap-2" },
        text({ style = "text-xs text-muted" }, "Volume"),
        slider({ value = state.vol, min = 0, max = 100, on_change = msg("set_vol") })
      )
    ))
  end
  return surfaces
end

function subscribe(state) return {} end
```

- [ ] **Step 5: Build and test manually**

Run: `cargo build --release && ./target/release/pulpkit examples/interactive/`
Verify: button click opens popup, slider drags, toggle works, click outside dismisses.

- [ ] **Step 6: Commit**

```bash
git add crates/pulpkit-core/ examples/interactive/
git commit -m "feat: interactive widgets — click, slider, toggle, popup lifecycle"
```

---

### Task 18: Scroll, Input, Each + Full Widget Set

**Files:**
- Modify: `crates/pulpkit-core/src/event_loop.rs`
- Modify: `crates/pulpkit-layout/src/paint.rs`
- Modify: `crates/pulpkit-layout/src/flex.rs`

Implement remaining widget behaviors.

- [ ] **Step 1: Add scroll handling**

On `PointerAxis` over a `Scroll` element: update scroll offset, queue damage for the scroll container's bounding box.

- [ ] **Step 2: Add text input handling**

On `KeyPress` when a text `Input` is focused: append UTF-8 char (or handle backspace), queue `on_input` message with new value.

- [ ] **Step 3: Verify each() keyed list rendering**

`each()` already produces `Each` elements with `KeyedChild` entries. Verify the tree diff correctly handles add/remove/reorder of keyed children. Write a test with a Lua script that changes the list between view() calls and verify the diff output.

- [ ] **Step 4: Commit**

```bash
git add crates/
git commit -m "feat: scroll, input, each() — complete v3 widget set"
```

---

### Task 19: Performance Validation

**Files:**
- No new files — this is a verification task

- [ ] **Step 1: Measure idle CPU**

Run the minimal example. Use `top -p $(pgrep pulpkit)` or `pidstat 1` to measure CPU. With only `interval(1000, "tick")` running, CPU between ticks should be 0.0%.

- [ ] **Step 2: Measure slider drag CPU**

Run the interactive example. Drag the slider continuously while monitoring CPU. Should be <1% (only the slider track region redraws, not the entire surface).

- [ ] **Step 3: Measure memory**

Check RSS with `ps aux | grep pulpkit` after 30s of running. Target: 25-30MB.

- [ ] **Step 4: Measure compile time**

```bash
cargo clean && time cargo build --release
```

Target: <30s (vs ~90s with skia-safe).

- [ ] **Step 5: Document results**

If any target is missed, create a follow-up task. Otherwise, note the measurements in the commit message.

- [ ] **Step 6: Commit any fixes**

```bash
git add -A
git commit -m "perf: validate v3 performance targets — [results]"
```
