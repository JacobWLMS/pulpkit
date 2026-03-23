//! Pulpkit Wayland integration — layer-shell surfaces, input, output tracking.

pub mod activation;
pub mod client;
pub mod data_control;
pub mod foreign_toplevel;
pub mod idle;
pub mod idle_notify;
pub mod input;
pub mod output;
pub mod output_management;
pub mod screencopy;
pub mod session_lock;
pub mod surface;
pub mod virtual_keyboard;

pub use client::{AppState, WaylandClient};
pub use input::InputEvent;
pub use output::OutputInfo;
pub use surface::{Anchor, Layer, LayerSurface, PopupAnchor, PopupSurface, SurfaceConfig, SurfaceMargins};
