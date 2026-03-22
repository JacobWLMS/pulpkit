//! Input handling for pointer and keyboard events.
//!
//! Provides an [`InputEvent`] enum that normalises sctk callbacks
//! into a simple event queue consumed by the shell runtime.

use wayland_client::backend::ObjectId;

/// An input event tied to a specific surface.
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
    /// A key was pressed.
    KeyPress {
        surface_id: ObjectId,
        /// The raw keycode.
        raw_code: u32,
        /// XKB keysym value.
        keysym: u32,
        /// UTF-8 text produced by this key (None for modifiers, function keys, etc).
        utf8: Option<String>,
    },
    /// A key was released.
    KeyRelease {
        surface_id: ObjectId,
        raw_code: u32,
        keysym: u32,
    },
    /// Keyboard focus left a surface (user clicked outside).
    KeyboardLeave {
        surface_id: ObjectId,
    },
}
