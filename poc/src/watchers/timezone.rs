use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use zbus::blocking::fdo::PropertiesProxy;
use zbus::blocking::MessageIterator;
use zbus::names::InterfaceName;
use zbus::zvariant::OwnedValue;

use crate::state::FullState;

const DEST: &str = "org.freedesktop.timedate1";
const PATH: &str = "/org/freedesktop/timedate1";
const IFACE: &str = "org.freedesktop.timedate1";

fn read_timezone(props: &PropertiesProxy, iface: &InterfaceName<'_>) -> Option<String> {
    let val: OwnedValue = props.get(iface.clone(), "Timezone").ok()?;
    String::try_from(val).ok()
}

fn update_timezone(
    props: &PropertiesProxy,
    iface: &InterfaceName<'_>,
    state: &Arc<Mutex<FullState>>,
    dirty: &Arc<AtomicBool>,
) {
    if let Some(tz) = read_timezone(props, iface) {
        if let Ok(mut s) = state.lock() {
            s.timezone = tz;
        }
        dirty.store(true, Ordering::Relaxed);
    }
}

pub fn start_timezone_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let conn = match zbus::blocking::Connection::system() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[pulpkit] timezone: failed to connect to system bus: {e}");
                return;
            }
        };

        let props = match PropertiesProxy::builder(&conn)
            .destination(DEST)
            .unwrap()
            .path(PATH)
            .unwrap()
            .build()
        {
            Ok(p) => p,
            Err(e) => {
                eprintln!("[pulpkit] timezone: failed to create proxy: {e}");
                return;
            }
        };

        let iface = InterfaceName::try_from(IFACE).unwrap();

        // Verify service is available
        if props.get(iface.clone(), "Timezone").is_err() {
            eprintln!("[pulpkit] timezone: service not available (org.freedesktop.timedate1)");
            return;
        }

        // Initial read
        update_timezone(&props, &iface, &state, &dirty);

        // Subscribe to PropertiesChanged on the timedate1 path
        let rule = zbus::MatchRule::builder()
            .msg_type(zbus::MessageType::Signal)
            .interface("org.freedesktop.DBus.Properties")
            .unwrap()
            .member("PropertiesChanged")
            .unwrap()
            .path(PATH)
            .unwrap()
            .build();

        let iter = match MessageIterator::for_match_rule(rule, &conn, Some(8)) {
            Ok(it) => it,
            Err(e) => {
                eprintln!("[pulpkit] timezone: failed to subscribe to signals: {e}");
                return;
            }
        };

        for msg in iter {
            match msg {
                Ok(_) => {
                    update_timezone(&props, &iface, &state, &dirty);
                }
                Err(e) => {
                    eprintln!("[pulpkit] timezone: signal iterator error: {e}");
                    break;
                }
            }
        }
    });
}
