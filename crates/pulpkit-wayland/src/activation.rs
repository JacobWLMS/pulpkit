//! XDG activation protocol — request window focus from another client.
//!
//! Wraps `xdg_activation_v1` for focus stealing prevention-aware activation.

use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols::xdg::activation::v1::client::{
    xdg_activation_token_v1::{self, XdgActivationTokenV1},
    xdg_activation_v1::XdgActivationV1,
};

use crate::AppState;

/// Bind the XDG activation global.
///
/// Returns `None` if the compositor does not support the protocol.
pub fn bind_xdg_activation(
    globals: &wayland_client::globals::GlobalList,
    qh: &QueueHandle<AppState>,
) -> Option<XdgActivationV1> {
    globals
        .bind::<XdgActivationV1, _, _>(qh, 1..=1, ())
        .ok()
}

impl Dispatch<XdgActivationV1, ()> for AppState {
    fn event(
        _state: &mut Self,
        _proxy: &XdgActivationV1,
        _event: <XdgActivationV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<XdgActivationTokenV1, ()> for AppState {
    fn event(
        _state: &mut Self,
        _proxy: &XdgActivationTokenV1,
        event: xdg_activation_token_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            xdg_activation_token_v1::Event::Done { token } => {
                log::debug!("Activation token received: {token}");
                // Store token for use with activate() call
            }
            _ => {}
        }
    }
}
