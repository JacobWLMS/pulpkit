//! Popup management — popups render on the bar surface, not as separate windows.
//!
//! When a popup opens, the bar surface expands to full screen. The popup
//! content is painted at a computed (x, y) position within the same buffer.
//! Clicks outside the popup area dismiss it. This matches pulp v2's
//! single-surface architecture.

use pulpkit_layout::{LayoutResult, Node, Theme, compute_layout, paint_tree};
use pulpkit_reactive::{DynValue, Signal};
use pulpkit_render::{Canvas, TextRenderer};
use pulpkit_wayland::PopupAnchor;

/// Static configuration for a popup.
pub struct PopupConfig {
    pub parent_name: String,
    pub anchor: PopupAnchor,
    pub offset: (i32, i32),
    pub dismiss_on_outside: bool,
    pub width: u32,
    pub height: u32,
    pub keyboard: bool,
}

/// A popup managed by the runtime. Rendered onto the bar surface.
pub struct ManagedPopup {
    pub name: String,
    pub root: Node,
    pub config: PopupConfig,
    pub visible_signal: Option<Signal<DynValue>>,
    pub on_key: Option<mlua::RegistryKey>,
    /// Computed absolute position within the full-screen surface.
    pub x: f32,
    pub y: f32,
    /// Cached layout from last render.
    pub layout: Option<LayoutResult>,
}

impl ManagedPopup {
    pub fn should_be_visible(&self) -> bool {
        self.visible_signal
            .as_ref()
            .map(|s| s.get().as_bool())
            .unwrap_or(false)
    }

    pub fn dismiss(&mut self) {
        if let Some(ref sig) = self.visible_signal {
            sig.set(DynValue::Bool(false));
        }
    }

    /// Compute the popup's absolute position within the full-screen surface.
    pub fn compute_position(&mut self, screen_w: u32, screen_h: u32, bar_h: u32) {
        let (ox, oy) = self.config.offset;
        let pw = self.config.width as i32;
        let ph = self.config.height as i32;
        let sw = screen_w as i32;
        let sh = screen_h as i32;
        let bh = bar_h as i32;

        let (x, y) = match self.config.anchor {
            PopupAnchor::TopRight => (sw - pw - ox, bh + oy),
            PopupAnchor::TopLeft => (ox, bh + oy),
            PopupAnchor::BottomRight => (sw - pw - ox, sh - ph - bh - oy),
            PopupAnchor::BottomLeft => (ox, sh - ph - bh - oy),
            PopupAnchor::Center => (
                (sw - pw) / 2 + ox,
                (sh - ph) / 2 + oy,
            ),
        };

        self.x = x.max(0) as f32;
        self.y = y.max(0) as f32;
        log::info!("Popup '{}' position: ({}, {}) size: {}x{}", self.name, self.x, self.y, pw, ph);
    }

    /// Render the popup content and return its layout.
    pub fn render(
        &mut self,
        canvas: &mut Canvas,
        text_renderer: &TextRenderer,
        theme: &Theme,
    ) {
        let layout = compute_layout(
            &self.root,
            self.config.width as f32,
            self.config.height as f32,
            text_renderer,
            &theme.font_family,
        );

        // Paint popup background + content at absolute position.
        canvas.save();
        canvas.translate(self.x, self.y);

        let bg = theme.colors.get("surface").copied().unwrap_or_default();
        canvas.draw_rounded_rect(0.0, 0.0, self.config.width as f32, self.config.height as f32, 0.0, bg);
        paint_tree(canvas, &layout, &theme.font_family);

        canvas.restore();

        self.layout = Some(layout);
    }

    /// Hit-test: is the point (in screen coords) inside this popup?
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.x + self.config.width as f32
            && y >= self.y && y < self.y + self.config.height as f32
    }

    /// Convert screen coords to popup-local coords for hit testing.
    pub fn to_local(&self, x: f32, y: f32) -> (f32, f32) {
        (x - self.x, y - self.y)
    }
}

/// Check if any popup is currently visible.
pub fn any_visible(popups: &[ManagedPopup]) -> bool {
    popups.iter().any(|p| p.should_be_visible())
}
