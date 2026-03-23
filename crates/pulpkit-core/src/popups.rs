//! Popup management — uses xdg_popup surfaces parented to the bar.
//!
//! Each popup gets its own xdg_popup surface created via SCTK. The bar
//! surface never changes. The compositor handles positioning, z-ordering,
//! and dismiss-on-click-outside via the xdg_popup protocol.

use pulpkit_layout::{compute_layout, paint_tree, LayoutResult, Node, Theme};
use pulpkit_reactive::{DynValue, Signal};
use pulpkit_render::{Canvas, Color, TextRenderer};
use pulpkit_wayland::{AppState, LayerSurface, PopupAnchor, PopupSurface};

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

/// A popup managed by the runtime.
pub struct ManagedPopup {
    pub name: String,
    pub root: Node,
    pub config: PopupConfig,
    pub visible_signal: Option<Signal<DynValue>>,
    pub on_key: Option<mlua::RegistryKey>,
    /// The xdg_popup surface — None when hidden.
    pub surface: Option<PopupSurface>,
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

    /// Create the xdg_popup surface, parented to the bar.
    pub fn show(
        &mut self,
        app_state: &mut AppState,
        parent: &LayerSurface,
        bar_width: u32,
        bar_height: u32,
        text_renderer: &TextRenderer,
        theme: &Theme,
    ) {
        if self.surface.is_some() {
            return;
        }

        // Compute anchor rect on the bar surface based on anchor type.
        let (ax, ay, aw, ah) = match self.config.anchor {
            PopupAnchor::TopRight => {
                // Right side of bar
                let x = bar_width as i32 - self.config.width as i32 - self.config.offset.0;
                (x.max(0), 0, self.config.width as i32, bar_height as i32)
            }
            PopupAnchor::TopLeft => {
                (self.config.offset.0, 0, self.config.width as i32, bar_height as i32)
            }
            PopupAnchor::Center => {
                // Center of bar — popup will drop below center
                let x = (bar_width as i32 - self.config.width as i32) / 2 + self.config.offset.0;
                (x.max(0), 0, self.config.width as i32, bar_height as i32)
            }
            _ => (0, 0, bar_width as i32, bar_height as i32),
        };

        match PopupSurface::new(
            app_state, parent,
            ax, ay, aw, ah,
            self.config.width, self.config.height,
        ) {
            Ok(surface) => {
                log::info!("Popup '{}' opened ({}x{} at anchor {},{} {}x{})",
                    self.name, self.config.width, self.config.height, ax, ay, aw, ah);
                self.surface = Some(surface);
            }
            Err(e) => {
                log::error!("Failed to create popup '{}': {e}", self.name);
            }
        }
    }

    /// Destroy the popup surface.
    pub fn hide(&mut self) {
        if self.surface.is_some() {
            log::info!("Popup '{}' closed", self.name);
            self.surface = None; // drop destroys the xdg_popup
            self.layout = None;
        }
    }

    /// Render popup content onto its own surface.
    pub fn render(&mut self, text_renderer: &TextRenderer, theme: &Theme) {
        let surface = match &mut self.surface {
            Some(s) => s,
            None => return,
        };

        let w = surface.width;
        let h = surface.height;

        let layout = compute_layout(
            &self.root,
            w as f32,
            h as f32,
            text_renderer,
            &theme.font_family,
        );

        let buf = surface.get_buffer();
        if let Some(mut canvas) = Canvas::from_buffer(buf, w as i32, h as i32) {
            let bg = theme.colors.get("surface").copied().unwrap_or_default();
            canvas.clear(bg);
            paint_tree(&mut canvas, &layout, &theme.font_family);
            canvas.flush();
        }

        surface.commit();
        self.layout = Some(layout);
    }

    /// Get the popup's surface ID for event matching.
    pub fn surface_id(&self) -> Option<wayland_client::backend::ObjectId> {
        self.surface.as_ref().map(|s| s.surface_id())
    }
}

/// Check if any popup is currently visible.
pub fn any_visible(popups: &[ManagedPopup]) -> bool {
    popups.iter().any(|p| p.should_be_visible())
}
