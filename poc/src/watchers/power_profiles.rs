use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use zbus::blocking::fdo::PropertiesProxy;
use zbus::blocking::MessageIterator;
use zbus::names::InterfaceName;
use zbus::zvariant::OwnedValue;

use crate::state::FullState;

const DEST: &str = "net.hadess.PowerProfiles";
const PATH: &str = "/net/hadess/PowerProfiles";
const IFACE: &str = "net.hadess.PowerProfiles";

fn read_active_profile(props: &PropertiesProxy, iface: &InterfaceName<'_>) -> Option<String> {
    let val: OwnedValue = props.get(iface.clone(), "ActiveProfile").ok()?;
    String::try_from(val).ok()
}

fn update_profile(
    props: &PropertiesProxy,
    iface: &InterfaceName<'_>,
    state: &Arc<Mutex<FullState>>,
    dirty: &Arc<AtomicBool>,
) {
    if let Some(profile) = read_active_profile(props, iface) {
        if let Ok(mut s) = state.lock() {
            s.power_profile = profile;
        }
        dirty.store(true, Ordering::Relaxed);
    }
}

pub fn start_power_profiles_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let conn = match zbus::blocking::Connection::system() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[pulpkit] power-profiles: failed to connect to system bus: {e}");
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
                eprintln!("[pulpkit] power-profiles: failed to create proxy: {e}");
                return;
            }
        };

        let iface = InterfaceName::try_from(IFACE).unwrap();

        // Verify service is available
        if props.get(iface.clone(), "ActiveProfile").is_err() {
            eprintln!("[pulpkit] power-profiles: service not available (net.hadess.PowerProfiles)");
            return;
        }

        // Initial read
        update_profile(&props, &iface, &state, &dirty);

        // Subscribe to PropertiesChanged on the power-profiles path
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
                eprintln!("[pulpkit] power-profiles: failed to subscribe to signals: {e}");
                return;
            }
        };

        for msg in iter {
            match msg {
                Ok(_) => {
                    update_profile(&props, &iface, &state, &dirty);
                }
                Err(e) => {
                    eprintln!("[pulpkit] power-profiles: signal iterator error: {e}");
                    break;
                }
            }
        }
    });
}
