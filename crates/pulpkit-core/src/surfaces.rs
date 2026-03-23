//! Managed surface — wraps a LayerSurface with its widget tree and layout.
//!
//! When popups are open, the surface expands to full screen. The bar content
//! renders at the top, popups render at computed positions, and the rest is
//! transparent (allowing click-through detection).

use std::cell::Cell;
use std::rc::Rc;

use pulpkit_layout::{compute_layout, paint_tree, LayoutResult, Node, Theme};
use pulpkit_render::{Canvas, Color, TextRenderer};
use pulpkit_wayland::LayerSurface;

use crate::popups::ManagedPopup;

/// A Wayland layer surface managed by the runtime.
pub struct ManagedSurface {
    pub name: String,
    pub surface: LayerSurface,
    pub root: Node,
    pub layout: Option<LayoutResult>,
    pub dirty: Rc<Cell<bool>>,
    pub hovered_node: Option<usize>,
    /// Original bar height (before expansion).
    pub bar_height: u32,
    /// Whether the surface is currently expanded to full screen.
    pub expanded: bool,
    /// Full screen dimensions (set from output info).
    pub screen_width: u32,
    pub screen_height: u32,
}

impl ManagedSurface {
    /// Expand the surface to full screen for popup rendering.
    pub fn expand(&mut self) {
        if self.expanded {
            return;
        }
        self.expanded = true;
        self.surface.anchor_full_screen();
        // Keep the bar's exclusive zone — don't change it, or windows rearrange.
        self.surface.set_keyboard_exclusive();
        // Request full screen size — don't resize buffer yet.
        // The compositor will send a configure with the correct dimensions.
        // The configure handler will resize the buffer and mark dirty.
        self.surface.sctk_layer().set_size(0, 0); // 0,0 = fill available space
        self.surface.commit_config();
        log::info!("Surface expand requested (waiting for configure)");
    }

    /// Shrink back to bar-only size.
    pub fn shrink(&mut self) {
        if !self.expanded {
            return;
        }
        self.expanded = false;
        self.surface.anchor_top();
        self.surface.set_exclusive_zone(self.bar_height as i32);
        self.surface.set_keyboard_none();
        // Request bar size — configure handler will resize buffer.
        self.surface.sctk_layer().set_size(0, self.bar_height);
        self.surface.commit_config();
        log::info!("Surface shrink requested");
    }

    /// Render the bar (and any visible popups) onto the surface.
    pub fn render_with_popups(
        &mut self,
        popups: &mut [ManagedPopup],
        text_renderer: &TextRenderer,
        theme: &Theme,
    ) {
        let w = self.surface.width();
        let h = self.surface.height();
        log::info!("render_with_popups: expanded={} surface={}x{}", self.expanded, w, h);

        // Lay out the bar content using actual surface width (handles fractional scaling).
        let bar_w = if self.expanded { w } else { w };
        let bar_layout = compute_layout(
            &self.root,
            bar_w as f32,
            self.bar_height as f32,
            text_renderer,
            &theme.font_family,
        );

        let buf = self.surface.get_buffer();
        let Some(mut canvas) = Canvas::from_buffer(buf, w as i32, h as i32) else {
            return;
        };

        if self.expanded {
            // Full screen: transparent background, bar at top, popups at positions.
            canvas.clear(Color::new(0, 0, 0, 0));
            // Draw bar background.
            let bg = theme.colors.get("base").copied().unwrap_or_default();
            canvas.draw_rounded_rect(0.0, 0.0, w as f32, self.bar_height as f32, 0.0, bg);
        } else {
            let bg = theme.colors.get("base").copied().unwrap_or_default();
            canvas.clear(bg);
        }

        // Paint bar content.
        paint_tree(&mut canvas, &bar_layout, &theme.font_family);

        // Paint visible popups.
        if self.expanded {
            for popup in popups.iter_mut() {
                if popup.should_be_visible() {
                    popup.render(&mut canvas, text_renderer, theme);
                }
            }
        }

        canvas.flush();
        self.surface.commit(); // attach buffer + commit
        self.layout = Some(bar_layout);
        self.dirty.set(false);
    }

    pub fn mark_dirty(&self) {
        self.dirty.set(true);
    }
}
