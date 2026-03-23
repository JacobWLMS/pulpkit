//! Session lock protocol — lock the session and display a lock screen.
//!
//! Wraps `ext_session_lock_v1` for implementing a screen locker.
//! The shell can lock the session, render lock surfaces on each output,
//! and unlock when authentication succeeds.

use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols::ext::session_lock::v1::client::{
    ext_session_lock_manager_v1::ExtSessionLockManagerV1,
    ext_session_lock_v1::{self, ExtSessionLockV1},
    ext_session_lock_surface_v1::{self, ExtSessionLockSurfaceV1},
};

use crate::AppState;

/// Bind the session lock manager from the global list.
pub fn bind_session_lock_manager(
    globals: &wayland_client::globals::GlobalList,
    qh: &QueueHandle<AppState>,
) -> Option<ExtSessionLockManagerV1> {
    globals
        .bind::<ExtSessionLockManagerV1, _, _>(qh, 1..=1, ())
        .ok()
}

impl Dispatch<ExtSessionLockManagerV1, ()> for AppState {
    fn event(
        _state: &mut Self,
        _proxy: &ExtSessionLockManagerV1,
        _event: <ExtSessionLockManagerV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ExtSessionLockV1, ()> for AppState {
    fn event(
        state: &mut Self,
        _proxy: &ExtSessionLockV1,
        event: ext_session_lock_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            ext_session_lock_v1::Event::Locked => {
                log::info!("Session locked");
                state.session_locked = true;
            }
            ext_session_lock_v1::Event::Finished => {
                log::info!("Session lock finished (compositor rejected or unlocked)");
                state.session_locked = false;
            }
            _ => {}
        }
    }
}

impl Dispatch<ExtSessionLockSurfaceV1, ()> for AppState {
    fn event(
        _state: &mut Self,
        _proxy: &ExtSessionLockSurfaceV1,
        event: ext_session_lock_surface_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            ext_session_lock_surface_v1::Event::Configure { serial, width, height } => {
                log::debug!("Lock surface configure: {width}x{height} (serial {serial})");
                // Acknowledge the configure
                _proxy.ack_configure(serial);
            }
            _ => {}
        }
    }
}
