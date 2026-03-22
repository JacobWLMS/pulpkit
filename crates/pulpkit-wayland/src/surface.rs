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
        WaylandSurface,
    },
    shm::slot::SlotPool,
};
use wayland_client::protocol::{wl_output, wl_shm};

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
}

/// A layer-shell surface with an associated shared-memory buffer for CPU rendering.
pub struct LayerSurface {
    layer: SctkLayerSurface,
    pool: SlotPool,
    width: u32,
    height: u32,
    /// Cached pixel buffer. Allocated on first `get_buffer` call or after resize.
    buffer_data: Vec<u8>,
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

        // Initial commit with no buffer — the compositor will respond with a configure.
        layer.commit();

        // Create shm buffer pool. Initial size accommodates the requested dimensions.
        let buf_size = (config.width as usize) * (config.height as usize) * 4;
        let pool_size = buf_size.max(256); // Minimum pool size
        let pool = SlotPool::new(pool_size, &state.shm)?;

        Ok(LayerSurface {
            layer,
            pool,
            width: config.width,
            height: config.height,
            buffer_data: vec![0u8; buf_size],
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

    /// Surface width in pixels.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Surface height in pixels.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Damage the full surface and commit the current buffer contents.
    ///
    /// This copies `buffer_data` into the wl_shm pool, attaches the buffer
    /// to the surface, marks the entire surface as damaged, and commits.
    pub fn commit(&mut self) {
        let stride = self.width as i32 * 4;
        let (buffer, canvas) = match self.pool.create_buffer(
            self.width as i32,
            self.height as i32,
            stride,
            wl_shm::Format::Argb8888,
        ) {
            Ok(pair) => pair,
            Err(e) => {
                log::error!("Failed to create shm buffer: {e}");
                return;
            }
        };

        // Copy our pixel data into the shm pool's canvas.
        let len = canvas.len().min(self.buffer_data.len());
        canvas[..len].copy_from_slice(&self.buffer_data[..len]);

        // Damage the entire surface.
        self.layer
            .wl_surface()
            .damage_buffer(0, 0, self.width as i32, self.height as i32);

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
        let buf_size = (width as usize) * (height as usize) * 4;
        self.buffer_data.resize(buf_size, 0);
        // Tell the compositor the new desired size.
        self.layer.set_size(width, height);
        self.layer.commit();
    }

    /// Access the underlying sctk LayerSurface (for advanced usage).
    pub fn sctk_layer(&self) -> &SctkLayerSurface {
        &self.layer
    }
}
