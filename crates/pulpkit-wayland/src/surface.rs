//! Layer-shell surface creation and management.
//!
//! Creates wlr-layer-shell surfaces backed by shared-memory buffers
//! suitable for CPU rendering (e.g. by Skia).

use smithay_client_toolkit::{
    shell::{
        wlr_layer::{
            Anchor as SctkAnchor, KeyboardInteractivity,
            Layer as SctkLayer, LayerSurface as SctkLayerSurface,
        },
        xdg::{XdgPositioner, popup::Popup},
        WaylandSurface,
    },
    shm::slot::SlotPool,
};
use wayland_client::{
    backend::ObjectId,
    protocol::{wl_output, wl_shm},
    Proxy,
};
use smithay_client_toolkit::reexports::protocols::xdg::shell::client::xdg_positioner;

use crate::client::AppState;

/// Which edge of the screen to anchor the surface to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    /// Anchor to top + left + right edges. Surface spans full width at the top.
    Top,
    /// Anchor to bottom + left + right edges. Surface spans full width at the bottom.
    Bottom,
    /// Anchor to left + top + bottom edges. Surface spans full height on the left.
    Left,
    /// Anchor to right + top + bottom edges. Surface spans full height on the right.
    Right,
}

impl Anchor {
    /// Convert to sctk's bitflag anchor value.
    fn to_sctk(self) -> SctkAnchor {
        match self {
            Anchor::Top => SctkAnchor::TOP | SctkAnchor::LEFT | SctkAnchor::RIGHT,
            Anchor::Bottom => SctkAnchor::BOTTOM | SctkAnchor::LEFT | SctkAnchor::RIGHT,
            Anchor::Left => SctkAnchor::LEFT | SctkAnchor::TOP | SctkAnchor::BOTTOM,
            Anchor::Right => SctkAnchor::RIGHT | SctkAnchor::TOP | SctkAnchor::BOTTOM,
        }
    }
}

/// Which layer to place the surface on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    Background,
    Bottom,
    Top,
    Overlay,
}

impl Layer {
    fn to_sctk(self) -> SctkLayer {
        match self {
            Layer::Background => SctkLayer::Background,
            Layer::Bottom => SctkLayer::Bottom,
            Layer::Top => SctkLayer::Top,
            Layer::Overlay => SctkLayer::Overlay,
        }
    }
}

/// Anchor for popup surfaces — allows corner and edge anchoring without
/// spanning the full width/height.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupAnchor {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    /// Centered on screen — no edge anchoring, positioned via margins.
    Center,
}

impl PopupAnchor {
    pub fn to_sctk(self) -> SctkAnchor {
        match self {
            PopupAnchor::TopLeft => SctkAnchor::TOP | SctkAnchor::LEFT,
            PopupAnchor::TopRight => SctkAnchor::TOP | SctkAnchor::RIGHT,
            PopupAnchor::BottomLeft => SctkAnchor::BOTTOM | SctkAnchor::LEFT,
            PopupAnchor::BottomRight => SctkAnchor::BOTTOM | SctkAnchor::RIGHT,
            // All 4 edges anchored + explicit size = compositor centers the surface.
            PopupAnchor::Center => SctkAnchor::TOP | SctkAnchor::BOTTOM | SctkAnchor::LEFT | SctkAnchor::RIGHT,
        }
    }
}

/// Margins for a layer surface (top, right, bottom, left).
#[derive(Debug, Clone, Copy, Default)]
pub struct SurfaceMargins {
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub left: i32,
}

/// Configuration for creating a layer surface.
#[derive(Debug, Clone)]
pub struct SurfaceConfig {
    /// Width of the surface in pixels.
    pub width: u32,
    /// Height of the surface in pixels.
    pub height: u32,
    /// Which screen edge(s) to anchor to.
    pub anchor: Anchor,
    /// Which layer to place the surface on.
    pub layer: Layer,
    /// Exclusive zone in pixels. For a top bar, set this to the bar height
    /// to reserve screen space. Set to -1 to not reserve any space.
    pub exclusive_zone: i32,
    /// Namespace string for the layer surface (e.g. "pulpkit-bar").
    pub namespace: String,
    /// Target a specific output, or `None` for the compositor's default.
    pub output: Option<wl_output::WlOutput>,
    /// Margins (for popup positioning). Defaults to zero on all sides.
    pub margins: SurfaceMargins,
}

/// A layer-shell surface with an associated shared-memory buffer for CPU rendering.
pub struct LayerSurface {
    layer: SctkLayerSurface,
    pool: SlotPool,
    width: u32,
    height: u32,
    /// Cached pixel buffer. Allocated on first `get_buffer` call or after resize.
    buffer_data: Vec<u8>,
    /// Buffer scale factor (1 or 2). Buffer pixel dimensions are width*scale × height*scale.
    pub scale: i32,
}

impl LayerSurface {
    /// Create a new layer surface from the given configuration.
    ///
    /// This creates the wl_surface, configures the layer shell properties
    /// (anchor, size, exclusive zone), and performs the initial commit
    /// so the compositor can respond with a configure event.
    pub fn new(state: &mut AppState, config: SurfaceConfig) -> anyhow::Result<Self> {
        let surface = state.compositor_state.create_surface(&state.qh);

        let layer = state.layer_shell.create_layer_surface(
            &state.qh,
            surface,
            config.layer.to_sctk(),
            Some(config.namespace),
            config.output.as_ref(),
        );

        layer.set_anchor(config.anchor.to_sctk());
        layer.set_exclusive_zone(config.exclusive_zone);
        layer.set_keyboard_interactivity(KeyboardInteractivity::None);
        layer.set_margin(
            config.margins.top,
            config.margins.right,
            config.margins.bottom,
            config.margins.left,
        );

        // Set size based on anchor direction.
        match config.anchor {
            Anchor::Top | Anchor::Bottom => {
                // Full width (0 = stretch), fixed height.
                layer.set_size(0, config.height);
            }
            Anchor::Left | Anchor::Right => {
                // Fixed width, full height (0 = stretch).
                layer.set_size(config.width, 0);
            }
        }

        // Use buffer_scale=1: render at native resolution, let compositor handle scaling.
        // Fractional scale (e.g. 1.25x) is handled by the compositor presenting our
        // buffer as-is at the logical size. For proper fractional-scale support,
        // we'd need the fractional-scale-v1 protocol.
        let scale = 1;
        layer.wl_surface().set_buffer_scale(scale);

        // Initial commit with no buffer — the compositor will respond with a configure.
        layer.commit();

        // Buffer at physical pixels (logical * scale).
        let phys_w = config.width as usize * scale as usize;
        let phys_h = config.height as usize * scale as usize;
        let buf_size = phys_w * phys_h * 4;
        let pool_size = buf_size.max(256);
        let pool = SlotPool::new(pool_size, &state.shm)?;

        Ok(LayerSurface {
            layer,
            pool,
            width: config.width,
            height: config.height,
            buffer_data: vec![0u8; buf_size],
            scale,
        })
    }

    /// Create a popup-style layer surface with corner anchoring and explicit size.
    ///
    /// Unlike `new`, this uses a `PopupAnchor` (corner-based) and sets an explicit
    /// size (no stretching). The surface is placed on the overlay layer with no
    /// exclusive zone.
    pub fn new_popup(
        state: &mut AppState,
        popup_anchor: PopupAnchor,
        width: u32,
        height: u32,
        margins: SurfaceMargins,
        namespace: String,
        output: Option<&wl_output::WlOutput>,
    ) -> anyhow::Result<Self> {
        Self::new_popup_with_keyboard(state, popup_anchor, width, height, margins, namespace, output, false)
    }

    /// Create a popup surface with optional keyboard interactivity.
    pub fn new_popup_with_keyboard(
        state: &mut AppState,
        popup_anchor: PopupAnchor,
        width: u32,
        height: u32,
        margins: SurfaceMargins,
        namespace: String,
        output: Option<&wl_output::WlOutput>,
        keyboard: bool,
    ) -> anyhow::Result<Self> {
        let surface = state.compositor_state.create_surface(&state.qh);

        let layer = state.layer_shell.create_layer_surface(
            &state.qh,
            surface,
            SctkLayer::Overlay,
            Some(namespace),
            output,
        );

        layer.set_anchor(popup_anchor.to_sctk());
        layer.set_exclusive_zone(-1);
        let kb = if keyboard {
            // Exclusive grabs keyboard focus — compositor sends keyboard leave
            // when user clicks outside, which we use for dismiss-on-outside.
            KeyboardInteractivity::Exclusive
        } else {
            KeyboardInteractivity::None
        };
        layer.set_keyboard_interactivity(kb);
        layer.set_margin(margins.top, margins.right, margins.bottom, margins.left);
        layer.set_size(width, height);

        let scale = 1;
        layer.wl_surface().set_buffer_scale(scale);

        layer.commit();

        let phys_w = width as usize * scale as usize;
        let phys_h = height as usize * scale as usize;
        let buf_size = phys_w * phys_h * 4;
        let pool_size = buf_size.max(256);
        let pool = SlotPool::new(pool_size, &state.shm)?;

        Ok(LayerSurface {
            layer,
            pool,
            width,
            height,
            buffer_data: vec![0u8; buf_size],
            scale,
        })
    }

    /// Create a full-screen transparent surface on the overlay layer.
    ///
    /// Used as a click-catcher behind popups: when the user clicks the backdrop,
    /// the popup dismisses. The surface is 1x1 pixel (stretched to full screen
    /// by the all-edges anchor) filled with transparent pixels.
    pub fn new_backdrop(
        state: &mut AppState,
        namespace: String,
        output: Option<&wl_output::WlOutput>,
    ) -> anyhow::Result<Self> {
        let surface = state.compositor_state.create_surface(&state.qh);

        let layer = state.layer_shell.create_layer_surface(
            &state.qh,
            surface,
            SctkLayer::Top, // Below Overlay so popups render on top
            Some(namespace),
            output,
        );

        // Anchor all edges = full screen. Size 0,0 = stretch to fill.
        layer.set_anchor(SctkAnchor::TOP | SctkAnchor::BOTTOM | SctkAnchor::LEFT | SctkAnchor::RIGHT);
        layer.set_exclusive_zone(-1);
        layer.set_keyboard_interactivity(KeyboardInteractivity::None);
        layer.set_size(0, 0);
        layer.commit();

        // Minimal 1x1 buffer — will be stretched by compositor.
        // Actually we need a real buffer. Use a small one filled with transparent.
        let width = 1u32;
        let height = 1u32;
        let buf_size = 4; // 1 pixel * 4 bytes
        let pool = SlotPool::new(256, &state.shm)?;

        Ok(LayerSurface {
            layer,
            pool,
            width,
            height,
            buffer_data: vec![0u8; buf_size], // transparent
            scale: 1,
        })
    }

    /// Get a mutable reference to the raw pixel buffer (ARGB8888 format).
    ///
    /// The buffer is `width * height * 4` bytes. Each pixel is 4 bytes
    /// in ARGB order (on little-endian this is BGRA in memory, matching
    /// what Skia expects for `kBGRA_8888`).
    ///
    /// After writing pixels, call [`commit`](Self::commit) to present.
    pub fn get_buffer(&mut self) -> &mut [u8] {
        &mut self.buffer_data
    }

    /// Surface logical width (what layout uses).
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Surface logical height (what layout uses).
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Buffer width in physical pixels (logical * scale).
    pub fn buffer_width(&self) -> u32 {
        self.width * self.scale as u32
    }

    /// Buffer height in physical pixels (logical * scale).
    pub fn buffer_height(&self) -> u32 {
        self.height * self.scale as u32
    }

    /// Commit the buffer. Buffer dimensions are physical (logical * scale).
    pub fn commit(&mut self) {
        let bw = self.buffer_width();
        let bh = self.buffer_height();
        let stride = bw as i32 * 4;
        let buf_size = (bw as usize) * (bh as usize) * 4;

        // Ensure pool is large enough — resize if needed (e.g. after surface resize).
        if self.pool.len() < buf_size {
            self.pool.resize(buf_size).ok();
        }

        let (buffer, canvas) = match self.pool.create_buffer(
            bw as i32,
            bh as i32,
            stride,
            wl_shm::Format::Argb8888,
        ) {
            Ok(pair) => pair,
            Err(_) => {
                if self.pool.resize(buf_size * 2).is_ok() {
                    match self.pool.create_buffer(
                        bw as i32,
                        bh as i32,
                        stride,
                        wl_shm::Format::Argb8888,
                    ) {
                        Ok(pair) => pair,
                        Err(e) => {
                            log::error!("Failed to create shm buffer after resize: {e}");
                            return;
                        }
                    }
                } else {
                    log::error!("Failed to resize shm pool");
                    return;
                }
            }
        };

        // Copy our pixel data into the shm pool's canvas.
        let len = canvas.len().min(self.buffer_data.len());
        canvas[..len].copy_from_slice(&self.buffer_data[..len]);

        // Damage the entire surface.
        self.layer
            .wl_surface()
            .damage_buffer(0, 0, bw as i32, bh as i32);

        // Attach buffer and commit.
        if let Err(e) = buffer.attach_to(self.layer.wl_surface()) {
            log::error!("Failed to attach buffer: {e}");
            return;
        }
        self.layer.commit();
    }

    /// Resize the surface. This updates the internal buffer and requests
    /// the compositor to resize via the layer shell protocol.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        let bw = width as usize * self.scale as usize;
        let bh = height as usize * self.scale as usize;
        let buf_size = bw * bh * 4;
        self.buffer_data.resize(buf_size, 0);
        self.layer.set_size(width, height);
        self.layer.commit();
    }

    /// Change the surface anchor at runtime using raw sctk anchor flags.
    pub fn set_anchor_raw(&mut self, anchor: SctkAnchor) {
        self.layer.set_anchor(anchor);
    }

    /// Expand surface to fill the entire screen (anchor all edges).
    pub fn anchor_full_screen(&mut self) {
        self.layer.set_anchor(
            SctkAnchor::TOP | SctkAnchor::BOTTOM | SctkAnchor::LEFT | SctkAnchor::RIGHT,
        );
    }

    /// Anchor surface to top edge only (span full width).
    pub fn anchor_top(&mut self) {
        self.layer.set_anchor(
            SctkAnchor::TOP | SctkAnchor::LEFT | SctkAnchor::RIGHT,
        );
    }

    /// Set keyboard interactivity: none, on-demand, or exclusive.
    pub fn set_keyboard_none(&mut self) {
        self.layer.set_keyboard_interactivity(KeyboardInteractivity::None);
    }

    pub fn set_keyboard_exclusive(&mut self) {
        self.layer.set_keyboard_interactivity(KeyboardInteractivity::Exclusive);
    }

    /// Set exclusive zone at runtime.
    pub fn set_exclusive_zone(&mut self, zone: i32) {
        self.layer.set_exclusive_zone(zone);
    }

    /// Return the Wayland `ObjectId` of this surface's `wl_surface`.
    ///
    /// This is used to match input events (which carry a surface `ObjectId`)
    /// to the correct managed surface.
    pub fn surface_id(&self) -> ObjectId {
        self.layer.wl_surface().id()
    }

    /// Commit protocol-level changes (anchor, size, zone, keyboard) to the compositor.
    /// Uses the underlying wl_surface commit to apply all pending state.
    pub fn commit_config(&self) {
        self.layer.wl_surface().commit();
    }

    /// Request a frame callback from the compositor.
    /// The compositor will send a frame event when it's ready for the next frame.
    pub fn request_frame(&self, qh: &wayland_client::QueueHandle<crate::client::AppState>) {
        self.layer.wl_surface().frame(qh, self.layer.wl_surface().clone());
    }

    /// Commit with per-region damage instead of full surface.
    pub fn commit_with_damage(&mut self, damage_rects: &[(i32, i32, i32, i32)]) {
        let bw = self.buffer_width();
        let bh = self.buffer_height();
        let stride = bw as i32 * 4;
        let buf_size = (bw as usize) * (bh as usize) * 4;

        if self.pool.len() < buf_size {
            self.pool.resize(buf_size).ok();
        }

        let (buffer, canvas) = match self.pool.create_buffer(
            bw as i32, bh as i32, stride,
            wl_shm::Format::Argb8888,
        ) {
            Ok(pair) => pair,
            Err(_) => {
                if self.pool.resize(buf_size * 2).is_ok() {
                    match self.pool.create_buffer(bw as i32, bh as i32, stride, wl_shm::Format::Argb8888) {
                        Ok(pair) => pair,
                        Err(e) => { log::error!("Failed to create shm buffer: {e}"); return; }
                    }
                } else {
                    log::error!("Failed to resize shm pool"); return;
                }
            }
        };

        let len = canvas.len().min(self.buffer_data.len());
        canvas[..len].copy_from_slice(&self.buffer_data[..len]);

        // Apply per-region damage instead of full surface
        if damage_rects.is_empty() {
            self.layer.wl_surface().damage_buffer(0, 0, bw as i32, bh as i32);
        } else {
            for &(x, y, w, h) in damage_rects {
                self.layer.wl_surface().damage_buffer(x, y, w, h);
            }
        }

        if let Err(e) = buffer.attach_to(self.layer.wl_surface()) {
            log::error!("Failed to attach buffer: {e}");
            return;
        }
        self.layer.commit();
    }

    /// Access the underlying sctk LayerSurface (for advanced usage).
    pub fn sctk_layer(&self) -> &SctkLayerSurface {
        &self.layer
    }
}

// ===========================================================================
// xdg_popup surface — child popup of a layer surface
// ===========================================================================

/// An xdg_popup surface parented to a layer-shell surface.
/// Gets its own pixel buffer for Skia rendering.
pub struct PopupSurface {
    popup: Popup,
    pool: SlotPool,
    pub width: u32,
    pub height: u32,
    buffer_data: Vec<u8>,
    /// Whether the compositor has sent the initial configure event.
    pub configured: bool,
}

impl PopupSurface {
    /// Create an xdg_popup parented to the given layer surface.
    ///
    /// `anchor_rect` is the rect on the parent surface that the popup anchors to
    /// (e.g., the button position). The popup appears below that rect.
    pub fn new(
        state: &mut AppState,
        parent: &LayerSurface,
        anchor_x: i32,
        anchor_y: i32,
        anchor_w: i32,
        anchor_h: i32,
        width: u32,
        height: u32,
        grab: bool,
    ) -> anyhow::Result<Self> {
        // Create positioner
        let positioner = XdgPositioner::new(&state.xdg_shell)
            .map_err(|e| anyhow::anyhow!("Failed to create positioner: {e}"))?;

        positioner.set_size(width as i32, height as i32);
        positioner.set_anchor_rect(anchor_x, anchor_y, anchor_w, anchor_h);
        positioner.set_anchor(xdg_positioner::Anchor::Bottom);
        positioner.set_gravity(xdg_positioner::Gravity::Bottom);
        positioner.set_constraint_adjustment(
            xdg_positioner::ConstraintAdjustment::SlideX
                | xdg_positioner::ConstraintAdjustment::SlideY
                | xdg_positioner::ConstraintAdjustment::FlipY,
        );

        // Create the popup surface
        let wl_surface = state.compositor_state.create_surface(&state.qh);
        let popup = Popup::from_surface(
            None, // NULL parent — will be reparented to layer surface
            &positioner,
            &state.qh,
            wl_surface,
            &state.xdg_shell,
        )
        .map_err(|e| anyhow::anyhow!("Failed to create popup: {e}"))?;

        // Reparent to the layer surface
        parent.sctk_layer().get_popup(popup.xdg_popup());

        // Grab keyboard + pointer for dismiss-on-outside and key input.
        if grab {
            if let Some(seat) = state.seat_state.seats().next() {
                popup.xdg_popup().grab(&seat, state.last_serial);
            }
        }

        // Initial commit — no buffer yet. Wait for configure event.
        // Don't set buffer_scale — let the compositor handle scaling
        // the same way it handles the bar surface.
        popup.wl_surface().commit();

        // Buffer at logical size (same as bar approach — compositor upscales).
        let buf_size = (width as usize) * (height as usize) * 4;
        let pool = SlotPool::new(buf_size.max(256), &state.shm)?;

        Ok(PopupSurface {
            popup,
            pool,
            width,
            height,
            buffer_data: vec![0u8; buf_size],
            configured: false,
        })
    }

    /// Get the pixel buffer for rendering.
    pub fn get_buffer(&mut self) -> &mut [u8] {
        &mut self.buffer_data
    }

    /// Commit the buffer to the compositor.
    pub fn commit(&mut self) {
        let stride = self.width as i32 * 4;
        let buf_size = (self.width as usize) * (self.height as usize) * 4;

        if self.pool.len() < buf_size {
            self.pool.resize(buf_size).ok();
        }

        let (buffer, canvas) = match self.pool.create_buffer(
            self.width as i32,
            self.height as i32,
            stride,
            wl_shm::Format::Argb8888,
        ) {
            Ok(pair) => pair,
            Err(_) => {
                if self.pool.resize(buf_size * 2).is_ok() {
                    match self.pool.create_buffer(
                        self.width as i32,
                        self.height as i32,
                        stride,
                        wl_shm::Format::Argb8888,
                    ) {
                        Ok(pair) => pair,
                        Err(e) => {
                            log::error!("Failed to create popup buffer: {e}");
                            return;
                        }
                    }
                } else {
                    log::error!("Failed to resize popup pool");
                    return;
                }
            }
        };

        let len = canvas.len().min(self.buffer_data.len());
        canvas[..len].copy_from_slice(&self.buffer_data[..len]);

        self.popup.wl_surface().damage_buffer(0, 0, self.width as i32, self.height as i32);

        if let Err(e) = buffer.attach_to(self.popup.wl_surface()) {
            log::error!("Failed to attach popup buffer: {e}");
            return;
        }
        self.popup.wl_surface().commit();
    }

    /// Get the surface ObjectId for event matching.
    pub fn surface_id(&self) -> ObjectId {
        self.popup.wl_surface().id()
    }
}
