//! Managed surface — wraps a LayerSurface with its widget tree and layout.

use std::cell::Cell;
use std::rc::Rc;

use pulpkit_layout::{compute_layout, paint_tree, LayoutResult, Node, Theme};
use pulpkit_render::{Canvas, TextRenderer};
use pulpkit_wayland::LayerSurface;

/// A Wayland layer surface managed by the runtime.
pub struct ManagedSurface {
    pub name: String,
    pub surface: LayerSurface,
    pub root: Node,
    pub layout: Option<LayoutResult>,
    pub dirty: Rc<Cell<bool>>,
    pub hovered_node: Option<usize>,
}

impl ManagedSurface {
    /// Render the widget tree onto the surface.
    pub fn render(&mut self, text_renderer: &TextRenderer, theme: &Theme) {
        let w = self.surface.width();   // logical
        let h = self.surface.height();  // logical
        let bw = self.surface.buffer_width();   // physical
        let bh = self.surface.buffer_height();  // physical
        let scale = self.surface.scale as f32;

        // Layout at logical dimensions.
        let layout = compute_layout(
            &self.root,
            w as f32,
            h as f32,
            text_renderer,
            &theme.font_family,
        );

        // Render at physical pixel dimensions with scale transform.
        let buf = self.surface.get_buffer();
        if let Some(mut canvas) = Canvas::from_buffer(buf, bw as i32, bh as i32) {
            let bg = theme.colors.get("base").copied().unwrap_or_default();
            canvas.clear(bg);
            if scale > 1.0 {
                canvas.save();
                canvas.scale(scale, scale);
            }
            paint_tree(&mut canvas, &layout, &theme.font_family);
            if scale > 1.0 {
                canvas.restore();
            }
            canvas.flush();
        }

        self.surface.commit();
        self.layout = Some(layout);
        self.dirty.set(false);
    }

    pub fn mark_dirty(&self) {
        self.dirty.set(true);
    }
}
