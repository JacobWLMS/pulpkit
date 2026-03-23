//! Data control protocol — clipboard access.
//!
//! Wraps `zwlr_data_control_manager_v1` for reading/writing clipboard contents
//! at the Wayland protocol level (no wl-paste/wl-copy dependency).

use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols_wlr::data_control::v1::client::{
    zwlr_data_control_device_v1::{self, ZwlrDataControlDeviceV1},
    zwlr_data_control_manager_v1::ZwlrDataControlManagerV1,
    zwlr_data_control_offer_v1::{self, ZwlrDataControlOfferV1},
    zwlr_data_control_source_v1::{self, ZwlrDataControlSourceV1},
};

use crate::AppState;

/// Bind the data control manager from the global list.
pub fn bind_data_control_manager(
    globals: &wayland_client::globals::GlobalList,
    qh: &QueueHandle<AppState>,
) -> Option<ZwlrDataControlManagerV1> {
    globals
        .bind::<ZwlrDataControlManagerV1, _, _>(qh, 1..=2, ())
        .ok()
}

impl Dispatch<ZwlrDataControlManagerV1, ()> for AppState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrDataControlManagerV1,
        _event: <ZwlrDataControlManagerV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwlrDataControlDeviceV1, ()> for AppState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrDataControlDeviceV1,
        event: zwlr_data_control_device_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_data_control_device_v1::Event::DataOffer { id: _ } => {
                log::debug!("Data control: new offer");
            }
            zwlr_data_control_device_v1::Event::Selection { id: _ } => {
                log::debug!("Data control: selection changed");
            }
            zwlr_data_control_device_v1::Event::PrimarySelection { id: _ } => {
                log::debug!("Data control: primary selection changed");
            }
            zwlr_data_control_device_v1::Event::Finished => {
                log::debug!("Data control: device finished");
            }
            _ => {}
        }
    }
}

impl Dispatch<ZwlrDataControlOfferV1, ()> for AppState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrDataControlOfferV1,
        event: zwlr_data_control_offer_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_data_control_offer_v1::Event::Offer { mime_type } => {
                log::debug!("Data control offer mime: {mime_type}");
            }
            _ => {}
        }
    }
}

impl Dispatch<ZwlrDataControlSourceV1, ()> for AppState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrDataControlSourceV1,
        event: zwlr_data_control_source_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_data_control_source_v1::Event::Send { mime_type: _, fd: _ } => {
                log::debug!("Data control: send requested");
            }
            zwlr_data_control_source_v1::Event::Cancelled => {
                log::debug!("Data control: source cancelled");
            }
            _ => {}
        }
    }
}
