use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use zbus::blocking::fdo::PropertiesProxy;
use zbus::blocking::MessageIterator;
use zbus::zvariant::OwnedValue;
use zbus::names::InterfaceName;

use crate::state::FullState;

const BAT_PATH: &str = "/org/freedesktop/UPower/devices/battery_BAT0";
const UPOWER_IFACE: &str = "org.freedesktop.UPower.Device";

fn state_to_string(state: u32) -> String {
    match state {
        1 => "Charging".into(),
        2 => "Discharging".into(),
        3 => "Empty".into(),
        4 => "Full".into(),
        5 => "Pending charge".into(),
        6 => "Pending discharge".into(),
        _ => "Unknown".into(),
    }
}

fn read_battery(
    props: &PropertiesProxy,
    iface: &InterfaceName<'_>,
    state: &Arc<Mutex<FullState>>,
    dirty: &Arc<AtomicBool>,
) {
    let pct: f64 = props
        .get(iface.clone(), "Percentage")
        .ok()
        .and_then(|v: OwnedValue| f64::try_from(v).ok())
        .unwrap_or(0.0);

    let bat_state: u32 = props
        .get(iface.clone(), "State")
        .ok()
        .and_then(|v: OwnedValue| u32::try_from(v).ok())
        .unwrap_or(0);

    let mut s = state.lock().unwrap();
    s.bat = pct as u32;
    s.bat_status = state_to_string(bat_state);
    s.has_bat = true;
    drop(s);
    dirty.store(true, Ordering::Relaxed);
}

pub fn start_upower_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let conn = match zbus::blocking::Connection::system() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[pulpkit] upower: failed to connect to system bus: {e}");
                return;
            }
        };

        // Build a properties proxy for the battery device
        let props = match PropertiesProxy::builder(&conn)
            .destination("org.freedesktop.UPower")
            .unwrap()
            .path(BAT_PATH)
            .unwrap()
            .build()
        {
            Ok(p) => p,
            Err(e) => {
                eprintln!("[pulpkit] upower: failed to create proxy for {BAT_PATH}: {e}");
                return;
            }
        };

        let iface = InterfaceName::try_from(UPOWER_IFACE).unwrap();

        // Verify the device actually exists by trying to read a property
        if props.get(iface.clone(), "Percentage").is_err() {
            eprintln!("[pulpkit] upower: battery device not available at {BAT_PATH}");
            return;
        }

        // Initial read
        read_battery(&props, &iface, &state, &dirty);

        // Build a match rule for PropertiesChanged signals on the battery path
        let rule = zbus::MatchRule::builder()
            .msg_type(zbus::MessageType::Signal)
            .interface("org.freedesktop.DBus.Properties")
            .unwrap()
            .member("PropertiesChanged")
            .unwrap()
            .path(BAT_PATH)
            .unwrap()
            .build();

        let iter = match MessageIterator::for_match_rule(rule, &conn, Some(8)) {
            Ok(it) => it,
            Err(e) => {
                eprintln!("[pulpkit] upower: failed to subscribe to signals: {e}");
                return;
            }
        };

        // Block on signals — each PropertiesChanged triggers a battery re-read
        for msg in iter {
            match msg {
                Ok(_) => {
                    read_battery(&props, &iface, &state, &dirty);
                }
                Err(e) => {
                    eprintln!("[pulpkit] upower: signal iterator error: {e}");
                    break;
                }
            }
        }
    });
}
