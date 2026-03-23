//! Virtual keyboard protocol — emulate keyboard input.
//!
//! Wraps `zwp_virtual_keyboard_v1` for on-screen keyboard support.

use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::{
    zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1,
    zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1,
};

use crate::AppState;

/// Bind the virtual keyboard manager from the global list.
///
/// Returns `None` if the compositor does not support the protocol.
pub fn bind_virtual_keyboard_manager(
    globals: &wayland_client::globals::GlobalList,
    qh: &QueueHandle<AppState>,
) -> Option<ZwpVirtualKeyboardManagerV1> {
    globals
        .bind::<ZwpVirtualKeyboardManagerV1, _, _>(qh, 1..=1, ())
        .ok()
}

impl Dispatch<ZwpVirtualKeyboardManagerV1, ()> for AppState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwpVirtualKeyboardManagerV1,
        _event: <ZwpVirtualKeyboardManagerV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwpVirtualKeyboardV1, ()> for AppState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwpVirtualKeyboardV1,
        _event: <ZwpVirtualKeyboardV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}
