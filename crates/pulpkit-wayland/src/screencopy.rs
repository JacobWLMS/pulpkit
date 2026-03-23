//! Screen copy protocol — capture screen contents.
//!
//! Wraps `zwlr_screencopy_manager_v1` for taking screenshots and screen recording.

use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols_wlr::screencopy::v1::client::{
    zwlr_screencopy_frame_v1::{self, ZwlrScreencopyFrameV1},
    zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1,
};

use crate::AppState;

/// Bind the screencopy manager from the global list.
pub fn bind_screencopy_manager(
    globals: &wayland_client::globals::GlobalList,
    qh: &QueueHandle<AppState>,
) -> Option<ZwlrScreencopyManagerV1> {
    globals
        .bind::<ZwlrScreencopyManagerV1, _, _>(qh, 1..=3, ())
        .ok()
}

impl Dispatch<ZwlrScreencopyManagerV1, ()> for AppState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrScreencopyManagerV1,
        _event: <ZwlrScreencopyManagerV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwlrScreencopyFrameV1, ()> for AppState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrScreencopyFrameV1,
        event: zwlr_screencopy_frame_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_screencopy_frame_v1::Event::Buffer { format, width, height, stride } => {
                log::debug!("Screencopy buffer: {width}x{height} stride={stride} format={format:?}");
            }
            zwlr_screencopy_frame_v1::Event::Ready { .. } => {
                log::debug!("Screencopy frame ready");
            }
            zwlr_screencopy_frame_v1::Event::Failed => {
                log::warn!("Screencopy frame failed");
            }
            _ => {}
        }
    }
}
