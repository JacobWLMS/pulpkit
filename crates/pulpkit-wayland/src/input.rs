//! Input handling for pointer events.
//!
//! Provides an [`InputEvent`] enum that normalises sctk pointer callbacks
//! into a simple event queue consumed by the shell runtime.

use wayland_client::backend::ObjectId;

/// A pointer/mouse input event tied to a specific surface.
#[derive(Debug, Clone)]
pub enum InputEvent {
    /// Pointer entered a surface.
    PointerEnter {
        surface_id: ObjectId,
        x: f64,
        y: f64,
    },
    /// Pointer left a surface.
    PointerLeave {
        surface_id: ObjectId,
    },
    /// Pointer moved within a surface.
    PointerMotion {
        surface_id: ObjectId,
        x: f64,
        y: f64,
    },
    /// A mouse button was pressed or released.
    PointerButton {
        surface_id: ObjectId,
        x: f64,
        y: f64,
        button: u32,
        pressed: bool,
    },
    /// Scroll / axis event.
    PointerAxis {
        surface_id: ObjectId,
        x: f64,
        y: f64,
        delta: f64,
        horizontal: bool,
    },
}
