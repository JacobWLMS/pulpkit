use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use zbus::blocking::Connection;
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};
use zbus::MatchRule;
use zbus::MessageType;

use crate::state::{BtDevice, FullState};

type ManagedObjects = HashMap<OwnedObjectPath, HashMap<String, HashMap<String, OwnedValue>>>;

/// Extract a string property from the interface properties map, returning empty string if missing.
fn prop_string(props: &HashMap<String, OwnedValue>, key: &str) -> String {
    props
        .get(key)
        .and_then(|v| {
            let val: Value = v.into();
            match val {
                Value::Str(s) => Some(s.to_string()),
                _ => None,
            }
        })
        .unwrap_or_default()
}

/// Extract a bool property from the interface properties map, returning false if missing.
fn prop_bool(props: &HashMap<String, OwnedValue>, key: &str) -> bool {
    props
        .get(key)
        .and_then(|v| {
            let val: Value = v.into();
            match val {
                Value::Bool(b) => Some(b),
                _ => None,
            }
        })
        .unwrap_or(false)
}

/// Read the adapter power state and connected devices from bluez.
fn read_bluetooth_state(conn: &Connection) -> (bool, Vec<BtDevice>) {
    // Read adapter powered state from /org/bluez/hci0
    let powered = match conn.call_method(
        Some("org.bluez"),
        "/org/bluez/hci0",
        Some("org.freedesktop.DBus.Properties"),
        "Get",
        &("org.bluez.Adapter1", "Powered"),
    ) {
        Ok(reply) => reply
            .body::<OwnedValue>()
            .ok()
            .and_then(|ov| {
                let val: Value = (&ov).into();
                match val {
                    Value::Bool(b) => Some(b),
                    _ => None,
                }
            })
            .unwrap_or(false),
        Err(_) => false,
    };

    // Get all managed objects from bluez
    let devices = match conn.call_method(
        Some("org.bluez"),
        "/",
        Some("org.freedesktop.DBus.ObjectManager"),
        "GetManagedObjects",
        &(),
    ) {
        Ok(reply) => {
            let objects: ManagedObjects = match reply.body() {
                Ok(o) => o,
                Err(e) => {
                    log::warn!("[bluetooth] failed to deserialize managed objects: {e}");
                    return (powered, vec![]);
                }
            };

            objects
                .iter()
                .filter_map(|(_path, ifaces)| {
                    let dev_props = ifaces.get("org.bluez.Device1")?;
                    let connected = prop_bool(dev_props, "Connected");
                    if !connected {
                        return None;
                    }
                    Some(BtDevice {
                        name: prop_string(dev_props, "Name"),
                        address: prop_string(dev_props, "Address"),
                        connected,
                        icon: prop_string(dev_props, "Icon"),
                    })
                })
                .collect()
        }
        Err(e) => {
            log::warn!("[bluetooth] GetManagedObjects failed: {e}");
            vec![]
        }
    };

    (powered, devices)
}

fn update_state(conn: &Connection, state: &Arc<Mutex<FullState>>, dirty: &Arc<AtomicBool>) {
    let (powered, devices) = read_bluetooth_state(conn);
    if let Ok(mut s) = state.lock() {
        s.bt_powered = powered;
        s.bt_connected = devices;
    }
    dirty.store(true, Ordering::Relaxed);
}

pub fn start_bluetooth_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let conn = match Connection::system() {
            Ok(c) => c,
            Err(e) => {
                log::warn!("[bluetooth] failed to connect to system bus: {e}");
                return;
            }
        };

        // Initial read
        update_state(&conn, &state, &dirty);

        // Set up match rule for all signals from org.bluez
        let rule = match MatchRule::builder()
            .msg_type(MessageType::Signal)
            .sender("org.bluez")
        {
            Ok(b) => b.build(),
            Err(e) => {
                log::warn!("[bluetooth] failed to build match rule: {e}");
                return;
            }
        };

        let mut iter =
            match zbus::blocking::MessageIterator::for_match_rule(rule, &conn, Some(64)) {
                Ok(it) => it,
                Err(e) => {
                    log::warn!("[bluetooth] failed to create message iterator: {e}");
                    return;
                }
            };

        // On any signal from bluez, re-read full state
        while let Some(Ok(_msg)) = iter.next() {
            update_state(&conn, &state, &dirty);
        }
    });
}
