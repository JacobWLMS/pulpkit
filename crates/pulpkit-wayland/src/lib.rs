//! Pulpkit Wayland integration.
//!
//! This crate wraps `smithay-client-toolkit` to provide layer-shell surface
//! creation on Wayland, output tracking, and a calloop-based event loop for
//! integration with the main runtime.

pub mod client;
pub mod input;
pub mod output;
pub mod surface;

// Re-export primary types at crate root for convenience.
pub use client::{AppState, WaylandClient};
pub use input::InputEvent;
pub use output::OutputInfo;
pub use surface::{Anchor, Layer, LayerSurface, PopupAnchor, SurfaceConfig, SurfaceMargins};
