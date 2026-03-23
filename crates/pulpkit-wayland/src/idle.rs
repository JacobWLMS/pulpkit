//! Idle inhibit protocol — prevents the compositor from entering idle state.
//!
//! Wraps `zwp_idle_inhibit_manager_v1` to create inhibitors tied to surfaces.
//! The compositor will not idle/blank the screen while an inhibitor is active
//! and its associated surface is visible.

use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols::wp::idle_inhibit::zv1::client::{
    zwp_idle_inhibit_manager_v1::ZwpIdleInhibitManagerV1,
    zwp_idle_inhibitor_v1::ZwpIdleInhibitorV1,
};

use crate::AppState;

/// Bind the idle inhibit manager from the global list.
///
/// Returns `None` if the compositor does not support the protocol.
pub fn bind_idle_inhibit_manager(
    globals: &wayland_client::globals::GlobalList,
    qh: &QueueHandle<AppState>,
) -> Option<ZwpIdleInhibitManagerV1> {
    globals
        .bind::<ZwpIdleInhibitManagerV1, _, _>(qh, 1..=1, ())
        .ok()
}

/// Create an idle inhibitor for the given surface.
///
/// While the inhibitor exists and the surface is visible, the compositor
/// will not enter idle state (no screen blanking, no lock screen).
///
/// Drop the returned `ZwpIdleInhibitorV1` or call `destroy()` on it to
/// release the inhibition.
pub fn create_inhibitor(
    manager: &ZwpIdleInhibitManagerV1,
    surface: &wayland_client::protocol::wl_surface::WlSurface,
    qh: &QueueHandle<AppState>,
) -> ZwpIdleInhibitorV1 {
    manager.create_inhibitor(surface, qh, ())
}

// Neither the manager nor the inhibitor emit any events,
// so these Dispatch impls are empty stubs.

impl Dispatch<ZwpIdleInhibitManagerV1, ()> for AppState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwpIdleInhibitManagerV1,
        _event: <ZwpIdleInhibitManagerV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwpIdleInhibitorV1, ()> for AppState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwpIdleInhibitorV1,
        _event: <ZwpIdleInhibitorV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}
