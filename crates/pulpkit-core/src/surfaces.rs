//! Managed surfaces — pairs a SurfaceDef with a Wayland LayerSurface.

use std::cell::Cell;

use pulpkit_layout::element::{Element, SurfaceDef};
use pulpkit_layout::flex::{LayoutResult, compute_layout};
use pulpkit_layout::paint::paint_tree;
use pulpkit_layout::damage::DamageRect;
use pulpkit_render::{Canvas, Color, TextRenderer};
use pulpkit_layout::Theme;
use pulpkit_wayland::LayerSurface;

/// A surface managed by the runtime — holds Wayland surface + layout state.
pub struct ManagedSurface {
    pub def: SurfaceDef,
    pub surface: LayerSurface,
    pub layout: Option<LayoutResult>,
    pub dirty: bool,
    pub frame_ready: bool,
}

impl ManagedSurface {
    /// Render the surface: layout + paint + commit.
    pub fn render(&mut self, text_renderer: &TextRenderer, theme: &Theme, hovered_node: Option<usize>) {
        let w = self.surface.width();
        let h = self.surface.height();
        if w == 0 || h == 0 {
            return;
        }

        let bw = self.surface.buffer_width();
        let bh = self.surface.buffer_height();
        let scale = self.surface.scale as f32;
        log::debug!("Render: logical={}x{}, buffer={}x{}, scale={}", w, h, bw, bh, scale);

        // Layout
        let elements = vec![self.def.root.clone()];
        let layout = compute_layout(&elements, w as f32, h as f32, text_renderer, &theme.font_family);

        // Paint into the buffer
        let buffer = self.surface.get_buffer();
        if let Some(mut canvas) = Canvas::from_buffer(buffer, bw, bh) {
            if scale > 1.0 {
                canvas.scale(scale, scale);
            }
            canvas.clear(Color::new(0, 0, 0, 0)); // transparent
            paint_tree(&mut canvas, &layout, &elements, &theme.font_family, text_renderer, None, hovered_node);
        }

        self.layout = Some(layout);
        self.surface.commit();
        self.dirty = false;
        self.frame_ready = false;
    }

    /// Mark as needing a repaint.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }
}
