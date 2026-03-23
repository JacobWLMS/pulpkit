//! Idle notification protocol — detects when the user is idle.
//!
//! Wraps `ext_idle_notification_v1` to get notified when the user has been
//! idle for a specified duration and when they resume activity.

use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols::ext::idle_notify::v1::client::{
    ext_idle_notifier_v1::ExtIdleNotifierV1,
    ext_idle_notification_v1::{self, ExtIdleNotificationV1},
};

use crate::AppState;

/// Bind the idle notifier from the global list.
///
/// Returns `None` if the compositor does not support the protocol.
pub fn bind_idle_notifier(
    globals: &wayland_client::globals::GlobalList,
    qh: &QueueHandle<AppState>,
) -> Option<ExtIdleNotifierV1> {
    globals
        .bind::<ExtIdleNotifierV1, _, _>(qh, 1..=1, ())
        .ok()
}

/// Create an idle notification that fires after `timeout_ms` milliseconds of inactivity.
///
/// The compositor will send `idled` when the user has been idle for the given duration,
/// and `resumed` when activity resumes.
pub fn create_idle_notification(
    notifier: &ExtIdleNotifierV1,
    timeout_ms: u32,
    seat: &wayland_client::protocol::wl_seat::WlSeat,
    qh: &QueueHandle<AppState>,
) -> ExtIdleNotificationV1 {
    notifier.get_idle_notification(timeout_ms, seat, qh, ())
}

impl Dispatch<ExtIdleNotifierV1, ()> for AppState {
    fn event(
        _state: &mut Self,
        _proxy: &ExtIdleNotifierV1,
        _event: <ExtIdleNotifierV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ExtIdleNotificationV1, ()> for AppState {
    fn event(
        state: &mut Self,
        _proxy: &ExtIdleNotificationV1,
        event: ext_idle_notification_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            ext_idle_notification_v1::Event::Idled => {
                log::info!("User is idle");
                state.idle = true;
            }
            ext_idle_notification_v1::Event::Resumed => {
                log::info!("User resumed");
                state.idle = false;
            }
            _ => {}
        }
    }
}
