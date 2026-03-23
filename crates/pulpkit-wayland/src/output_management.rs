//! Output management protocol — configure displays.
//!
//! Wraps `zwlr_output_manager_v1` for display hotplug and configuration.
//! Provides output modes, positions, scales, and transform info.

use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols_wlr::output_management::v1::client::{
    zwlr_output_configuration_v1::{self, ZwlrOutputConfigurationV1},
    zwlr_output_configuration_head_v1::ZwlrOutputConfigurationHeadV1,
    zwlr_output_head_v1::{self, ZwlrOutputHeadV1},
    zwlr_output_manager_v1::{self, ZwlrOutputManagerV1},
    zwlr_output_mode_v1::{self, ZwlrOutputModeV1},
};

use crate::AppState;

/// Bind the output manager from the global list.
pub fn bind_output_manager(
    globals: &wayland_client::globals::GlobalList,
    qh: &QueueHandle<AppState>,
) -> Option<ZwlrOutputManagerV1> {
    globals
        .bind::<ZwlrOutputManagerV1, _, _>(qh, 1..=4, ())
        .ok()
}

impl Dispatch<ZwlrOutputManagerV1, ()> for AppState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrOutputManagerV1,
        event: zwlr_output_manager_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_output_manager_v1::Event::Head { head: _ } => {
                log::debug!("New output head");
            }
            zwlr_output_manager_v1::Event::Done { serial } => {
                log::debug!("Output manager done (serial {serial})");
            }
            zwlr_output_manager_v1::Event::Finished => {
                log::info!("Output manager finished");
            }
            _ => {}
        }
    }
}

impl Dispatch<ZwlrOutputHeadV1, ()> for AppState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrOutputHeadV1,
        event: zwlr_output_head_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_output_head_v1::Event::Name { name } => {
                log::debug!("Output head name: {name}");
            }
            zwlr_output_head_v1::Event::Description { description } => {
                log::debug!("Output head description: {description}");
            }
            zwlr_output_head_v1::Event::Enabled { enabled } => {
                log::debug!("Output head enabled: {enabled}");
            }
            _ => {}
        }
    }
}

impl Dispatch<ZwlrOutputModeV1, ()> for AppState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrOutputModeV1,
        event: zwlr_output_mode_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_output_mode_v1::Event::Size { width, height } => {
                log::debug!("Output mode: {width}x{height}");
            }
            _ => {}
        }
    }
}

impl Dispatch<ZwlrOutputConfigurationV1, ()> for AppState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrOutputConfigurationV1,
        event: zwlr_output_configuration_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_output_configuration_v1::Event::Succeeded => {
                log::info!("Output configuration applied");
            }
            zwlr_output_configuration_v1::Event::Failed => {
                log::warn!("Output configuration failed");
            }
            zwlr_output_configuration_v1::Event::Cancelled => {
                log::info!("Output configuration cancelled");
            }
            _ => {}
        }
    }
}

impl Dispatch<ZwlrOutputConfigurationHeadV1, ()> for AppState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrOutputConfigurationHeadV1,
        _event: <ZwlrOutputConfigurationHeadV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}
