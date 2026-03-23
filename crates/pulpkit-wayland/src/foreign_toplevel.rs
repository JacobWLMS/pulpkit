//! Foreign toplevel management — list and control opened windows.
//!
//! Wraps `zwlr_foreign_toplevel_manager_v1` for compositor-agnostic window tracking.
//! Provides window title, app_id, and state (maximized, minimized, activated, fullscreen).

use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols_wlr::foreign_toplevel::v1::client::{
    zwlr_foreign_toplevel_handle_v1::{self, ZwlrForeignToplevelHandleV1},
    zwlr_foreign_toplevel_manager_v1::{self, ZwlrForeignToplevelManagerV1},
};

use crate::AppState;

/// Bind the foreign toplevel manager from the global list.
///
/// Returns `None` if the compositor does not support the protocol.
pub fn bind_foreign_toplevel_manager(
    globals: &wayland_client::globals::GlobalList,
    qh: &QueueHandle<AppState>,
) -> Option<ZwlrForeignToplevelManagerV1> {
    globals
        .bind::<ZwlrForeignToplevelManagerV1, _, _>(qh, 1..=3, ())
        .ok()
}

/// A tracked toplevel window.
#[derive(Debug, Clone)]
pub struct ToplevelInfo {
    pub title: String,
    pub app_id: String,
    pub activated: bool,
    pub maximized: bool,
    pub minimized: bool,
    pub fullscreen: bool,
}

impl Dispatch<ZwlrForeignToplevelManagerV1, ()> for AppState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrForeignToplevelManagerV1,
        event: zwlr_foreign_toplevel_manager_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_foreign_toplevel_manager_v1::Event::Toplevel { toplevel: _ } => {
                // A new toplevel was created — events will follow on the handle.
                log::debug!("New foreign toplevel");
            }
            zwlr_foreign_toplevel_manager_v1::Event::Finished => {
                log::info!("Foreign toplevel manager finished");
            }
            _ => {}
        }
    }
}

impl Dispatch<ZwlrForeignToplevelHandleV1, ()> for AppState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrForeignToplevelHandleV1,
        event: zwlr_foreign_toplevel_handle_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_foreign_toplevel_handle_v1::Event::Title { title } => {
                log::debug!("Toplevel title: {title}");
            }
            zwlr_foreign_toplevel_handle_v1::Event::AppId { app_id } => {
                log::debug!("Toplevel app_id: {app_id}");
            }
            zwlr_foreign_toplevel_handle_v1::Event::State { state: _ } => {
                // state is a Vec<u8> encoding the toplevel states
            }
            zwlr_foreign_toplevel_handle_v1::Event::Done => {
                // All properties for this toplevel have been sent
            }
            zwlr_foreign_toplevel_handle_v1::Event::Closed => {
                log::debug!("Toplevel closed");
            }
            _ => {}
        }
    }
}
