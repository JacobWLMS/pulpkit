//! Popup state machine — manages popup lifecycle with fade animations.

use pulpkit_layout::{
    compute_layout, paint_tree, FadeAnimation, LayoutResult, Node, Theme,
};
use pulpkit_reactive::{DynValue, Signal};
use pulpkit_render::{Canvas, TextRenderer};
use pulpkit_wayland::{AppState, LayerSurface, PopupAnchor, SurfaceMargins};

/// Static configuration for a popup surface.
pub struct PopupConfig {
    pub parent_name: String,
    pub anchor: PopupAnchor,
    pub offset: (i32, i32),
    pub dismiss_on_outside: bool,
    pub width: u32,
    pub height: u32,
    pub output: Option<pulpkit_wayland::OutputInfo>,
    pub keyboard: bool,
}

/// The lifecycle state of a popup surface.
pub enum PopupState {
    /// No surface exists.
    Hidden,
    /// Surface created, waiting for compositor configure.
    Creating { surface: LayerSurface },
    /// Fading in after first configure.
    FadingIn {
        surface: LayerSurface,
        animation: FadeAnimation,
        layout: LayoutResult,
    },
    /// Fully visible and interactive.
    Visible {
        surface: LayerSurface,
        layout: LayoutResult,
    },
    /// Fading out before surface destruction.
    FadingOut {
        surface: LayerSurface,
        animation: FadeAnimation,
    },
}

impl PopupState {
    /// Whether the popup is currently animating (needs 60fps ticks).
    pub fn is_animating(&self) -> bool {
        matches!(self, PopupState::FadingIn { .. } | PopupState::FadingOut { .. })
    }
}

/// A popup managed by the runtime.
pub struct ManagedPopup {
    pub name: String,
    pub root: Node,
    pub state: PopupState,
    pub config: PopupConfig,
    pub visible_signal: Option<Signal<DynValue>>,
    /// Registry key for the on_key Lua callback (if keyboard enabled).
    pub on_key: Option<mlua::RegistryKey>,
    /// Full-screen transparent surface behind the popup for click-to-dismiss.
    pub backdrop: Option<LayerSurface>,
}

impl ManagedPopup {
    /// Whether the popup's visibility signal says it should be visible.
    pub fn should_be_visible(&self) -> bool {
        self.visible_signal
            .as_ref()
            .map(|s| s.get().as_bool())
            .unwrap_or(false)
    }

    /// Create the Wayland surface and transition to Creating state.
    ///
    /// Uses the popup's configured anchor to let the compositor handle
    /// corner/edge placement. Margins offset from the anchor point.
    pub fn show(
        &mut self,
        app_state: &mut AppState,
        parent_height: u32,
        text_renderer: &TextRenderer,
        theme: &Theme,
    ) {
        if !matches!(self.state, PopupState::Hidden) {
            return;
        }

        let popup_width = self.config.width;
        let popup_height = self.config.height;

        // Margins: offset from the anchor edge.
        // The anchor determines which corner/edge the popup is attached to.
        // Margins push it away from that corner.
        let (ox, oy) = self.config.offset;
        let bar_h = parent_height as i32;
        let margins = match self.config.anchor {
            PopupAnchor::TopLeft => SurfaceMargins {
                top: bar_h + oy,
                left: ox.max(0),
                right: 0, bottom: 0,
            },
            PopupAnchor::TopRight => SurfaceMargins {
                top: bar_h + oy,
                right: ox.max(0),
                left: 0, bottom: 0,
            },
            PopupAnchor::BottomLeft => SurfaceMargins {
                bottom: bar_h + oy,
                left: ox.max(0),
                right: 0, top: 0,
            },
            PopupAnchor::BottomRight => SurfaceMargins {
                bottom: bar_h + oy,
                right: ox.max(0),
                left: 0, top: 0,
            },
            PopupAnchor::Center => {
                // Center: no anchor edges. We use margins to center manually.
                // The compositor doesn't center for us — we compute it.
                let (sw, sh) = self.config.output
                    .as_ref()
                    .map(|o| (o.logical_width() as i32, o.logical_height() as i32))
                    .unwrap_or((1920, 1080));
                SurfaceMargins {
                    top: ((sh - popup_height as i32) / 2 + oy).max(0),
                    left: ((sw - popup_width as i32) / 2 + ox).max(0),
                    right: 0, bottom: 0,
                }
            }
        };

        // Use the REAL anchor — let the compositor handle edge placement.
        // Center uses TopLeft since we compute margins manually.
        let anchor = if self.config.anchor == PopupAnchor::Center {
            PopupAnchor::TopLeft
        } else {
            self.config.anchor
        };
        log::info!(
            "Popup '{}': anchor={:?} size={}x{} margins=t:{} r:{} b:{} l:{}",
            self.name, anchor, popup_width, popup_height,
            margins.top, margins.right, margins.bottom, margins.left,
        );
        match LayerSurface::new_popup_with_keyboard(
            app_state,
            anchor,
            popup_width,
            popup_height,
            margins,
            format!("pulpkit-popup-{}", self.name),
            self.config.output.as_ref().map(|o| &o.wl_output),
            self.config.keyboard,
        ) {
            Ok(surface) => {
                log::info!(
                    "Showing popup '{}' ({}x{})",
                    self.name, popup_width, popup_height,
                );
                // Backdrop disabled — it causes z-order and focus issues.
                // Dismiss-on-outside handled by keyboard leave + pointer leave.
                self.state = PopupState::Creating { surface };
            }
            Err(e) => {
                log::error!("Failed to create popup surface '{}': {e}", self.name);
            }
        }
    }

    /// Start fade-out animation.
    pub fn hide(&mut self) {
        // Destroy backdrop immediately.
        self.backdrop = None;

        match std::mem::replace(&mut self.state, PopupState::Hidden) {
            PopupState::Visible { surface, .. }
            | PopupState::FadingIn { surface, .. } => {
                log::info!("Hiding popup '{}' (fade-out)", self.name);
                self.state = PopupState::FadingOut {
                    surface,
                    animation: FadeAnimation::new(1.0, 0.0, 150),
                };
            }
            other => {
                // Put it back if we can't hide from this state.
                self.state = other;
            }
        }
    }

    /// Handle a compositor configure event for this popup's surface.
    ///
    /// Transitions Creating -> FadingIn (renders initial frame).
    pub fn handle_configure(
        &mut self,
        width: u32,
        height: u32,
        text_renderer: &TextRenderer,
        theme: &Theme,
    ) {
        match std::mem::replace(&mut self.state, PopupState::Hidden) {
            PopupState::Creating { mut surface } => {
                if width > 0 && height > 0 {
                    surface.resize(width, height);
                }
                let layout = render_popup_surface(
                    &mut surface,
                    &self.root,
                    text_renderer,
                    theme,
                    Some(0.0),
                );
                self.state = PopupState::FadingIn {
                    surface,
                    animation: FadeAnimation::new(0.0, 1.0, 200),
                    layout,
                };
            }
            other => {
                self.state = other;
            }
        }
    }

    /// Advance animation state. Returns true if still animating.
    pub fn tick(&mut self, text_renderer: &TextRenderer, theme: &Theme) -> bool {
        match std::mem::replace(&mut self.state, PopupState::Hidden) {
            PopupState::FadingIn {
                mut surface,
                animation,
                layout,
            } => {
                let (opacity, done) = animation.current();
                render_popup_surface(
                    &mut surface,
                    &self.root,
                    text_renderer,
                    theme,
                    Some(opacity),
                );
                if done {
                    self.state = PopupState::Visible { surface, layout };
                } else {
                    self.state = PopupState::FadingIn {
                        surface,
                        animation,
                        layout,
                    };
                }
                !done
            }
            PopupState::FadingOut {
                mut surface,
                animation,
            } => {
                let (opacity, done) = animation.current();
                if done {
                    log::info!(
                        "Popup '{}' fade-out complete, destroying surface",
                        self.name
                    );
                    // Surface is dropped here, destroying the Wayland surface.
                    drop(surface);
                    self.state = PopupState::Hidden;
                    false
                } else {
                    render_popup_surface(
                        &mut surface,
                        &self.root,
                        text_renderer,
                        theme,
                        Some(opacity),
                    );
                    self.state = PopupState::FadingOut { surface, animation };
                    true
                }
            }
            other => {
                self.state = other;
                false
            }
        }
    }

    /// Dismiss the popup by setting its visibility signal to false.
    pub fn dismiss(&mut self) {
        if let Some(ref sig) = self.visible_signal {
            sig.set(DynValue::Bool(false));
        }
    }

    /// Return the Wayland surface ObjectId, if a surface exists.
    pub fn surface_id(&self) -> Option<wayland_client::backend::ObjectId> {
        match &self.state {
            PopupState::Creating { surface }
            | PopupState::FadingIn { surface, .. }
            | PopupState::Visible { surface, .. }
            | PopupState::FadingOut { surface, .. } => Some(surface.surface_id()),
            PopupState::Hidden => None,
        }
    }

    /// Return the layout, if available (only in FadingIn or Visible states).
    pub fn layout(&self) -> Option<&LayoutResult> {
        match &self.state {
            PopupState::FadingIn { layout, .. } | PopupState::Visible { layout, .. } => {
                Some(layout)
            }
            _ => None,
        }
    }

    /// Re-render the popup's content (used when reactive state changes).
    pub fn render_content(&mut self, text_renderer: &TextRenderer, theme: &Theme) {
        match &mut self.state {
            PopupState::Visible { surface, layout } => {
                *layout = render_popup_surface(
                    surface,
                    &self.root,
                    text_renderer,
                    theme,
                    None,
                );
            }
            PopupState::FadingIn {
                surface,
                animation,
                layout,
            } => {
                let (opacity, _) = animation.current();
                *layout = render_popup_surface(
                    surface,
                    &self.root,
                    text_renderer,
                    theme,
                    Some(opacity),
                );
            }
            _ => {}
        }
    }
}

/// Render a popup surface with optional opacity (for fade animations).
fn render_popup_surface(
    surface: &mut LayerSurface,
    root: &Node,
    text_renderer: &TextRenderer,
    theme: &Theme,
    opacity: Option<f32>,
) -> LayoutResult {
    let w = surface.width();
    let h = surface.height();

    let layout = compute_layout(root, w as f32, h as f32, text_renderer, &theme.font_family);

    let buf = surface.get_buffer();
    if let Some(mut canvas) = Canvas::from_buffer(buf, w as i32, h as i32) {
        // Clear to transparent — the Lua content provides its own background
        // (e.g., bg-surface with rounded corners). A solid base fill would
        // create a visible rectangle behind the rounded popup.
        canvas.clear(pulpkit_render::Color::new(0, 0, 0, 0));
        paint_tree(&mut canvas, &layout, &theme.font_family);
        canvas.flush();
    }

    if let Some(opacity) = opacity {
        apply_opacity(surface.get_buffer(), opacity);
    }

    surface.commit();
    layout
}

/// Apply a global opacity to a pixel buffer in ARGB8888 (BGRA in memory) format.
///
/// Multiplies every channel (B, G, R, A) by the opacity factor. This is correct
/// for premultiplied alpha: both color and alpha channels must be scaled together.
pub fn apply_opacity(buffer: &mut [u8], opacity: f32) {
    if opacity >= 1.0 {
        return;
    }
    let alpha_mult = (opacity.clamp(0.0, 1.0) * 255.0) as u32;
    for pixel in buffer.chunks_exact_mut(4) {
        pixel[0] = ((pixel[0] as u32 * alpha_mult) / 255) as u8;
        pixel[1] = ((pixel[1] as u32 * alpha_mult) / 255) as u8;
        pixel[2] = ((pixel[2] as u32 * alpha_mult) / 255) as u8;
        pixel[3] = ((pixel[3] as u32 * alpha_mult) / 255) as u8;
    }
}

/// Compute margins for a popup surface based on anchor, parent height, and offset.
fn compute_popup_margins(
    anchor: PopupAnchor,
    parent_height: u32,
    offset: (i32, i32),
) -> SurfaceMargins {
    match anchor {
        PopupAnchor::TopLeft => SurfaceMargins {
            top: parent_height as i32 + offset.1,
            left: offset.0.max(0),
            right: 0,
            bottom: 0,
        },
        PopupAnchor::TopRight => SurfaceMargins {
            top: parent_height as i32 + offset.1,
            right: offset.0.abs(),
            left: 0,
            bottom: 0,
        },
        PopupAnchor::BottomLeft => SurfaceMargins {
            bottom: parent_height as i32 + offset.1,
            left: offset.0.max(0),
            right: 0,
            top: 0,
        },
        PopupAnchor::BottomRight => SurfaceMargins {
            bottom: parent_height as i32 + offset.1,
            right: offset.0.abs(),
            left: 0,
            top: 0,
        },
        PopupAnchor::Center => SurfaceMargins {
            top: offset.1,
            left: offset.0,
            right: 0,
            bottom: 0,
        },
    }
}
