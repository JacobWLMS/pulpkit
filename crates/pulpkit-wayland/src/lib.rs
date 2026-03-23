//! Pulpkit Wayland integration — layer-shell surfaces, input, output tracking.

pub mod client;
pub mod input;
pub mod output;
pub mod surface;

pub use client::{AppState, WaylandClient};
pub use input::InputEvent;
pub use output::OutputInfo;
pub use surface::{Anchor, Layer, LayerSurface, PopupAnchor, PopupSurface, SurfaceConfig, SurfaceMargins};
