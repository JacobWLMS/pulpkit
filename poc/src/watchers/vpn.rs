use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use zbus::blocking::Connection;
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};
use zbus::MatchRule;
use zbus::MessageType;

use crate::poll::sh;
use crate::state::FullState;

fn read_vpn_state(conn: &Connection) -> (bool, String) {
    // Get active connections from NetworkManager
    let active_paths: Vec<OwnedObjectPath> = match conn.call_method(
        Some("org.freedesktop.NetworkManager"),
        "/org/freedesktop/NetworkManager",
        Some("org.freedesktop.DBus.Properties"),
        "Get",
        &("org.freedesktop.NetworkManager", "ActiveConnections"),
    ) {
        Ok(reply) => {
            let ov: OwnedValue = match reply.body() {
                Ok(v) => v,
                Err(_) => return (false, String::new()),
            };
            let val: Value = (&ov).into();
            match val {
                Value::Array(arr) => arr
                    .iter()
                    .filter_map(|v| {
                        if let Value::ObjectPath(p) = v {
                            Some(OwnedObjectPath::from(p.clone()))
                        } else {
                            None
                        }
                    })
                    .collect(),
                _ => return (false, String::new()),
            }
        }
        Err(_) => return (false, String::new()),
    };

    // Check each active connection for VPN/WireGuard type
    for path in &active_paths {
        let conn_type = get_string_property(conn, path.as_str(), "Type");
        if conn_type == "vpn" || conn_type == "wireguard" {
            let name = get_string_property(conn, path.as_str(), "Id");
            return (true, name);
        }
    }

    (false, String::new())
}

fn get_string_property(conn: &Connection, path: &str, property: &str) -> String {
    match conn.call_method(
        Some("org.freedesktop.NetworkManager"),
        path,
        Some("org.freedesktop.DBus.Properties"),
        "Get",
        &(
            "org.freedesktop.NetworkManager.Connection.Active",
            property,
        ),
    ) {
        Ok(reply) => {
            let ov: OwnedValue = match reply.body() {
                Ok(v) => v,
                Err(_) => return String::new(),
            };
            let val: Value = (&ov).into();
            match val {
                Value::Str(s) => s.to_string(),
                _ => String::new(),
            }
        }
        Err(_) => String::new(),
    }
}

/// Fallback: check for wireguard interfaces via `ip link`
fn read_vpn_fallback() -> (bool, String) {
    if let Some(output) = sh("ip link show type wireguard 2>/dev/null") {
        // Parse interface name from output like "4: wg0: <POINTOPOINT,..."
        for line in output.lines() {
            if let Some(name) = line.split(':').nth(1) {
                let name = name.trim();
                if !name.is_empty() {
                    return (true, name.to_string());
                }
            }
        }
    }
    (false, String::new())
}

fn update_state(
    conn: Option<&Connection>,
    state: &Arc<Mutex<FullState>>,
    dirty: &Arc<AtomicBool>,
) {
    let (active, name) = match conn {
        Some(c) => {
            let (a, n) = read_vpn_state(c);
            if !a {
                // Double-check with fallback even when NM is available
                read_vpn_fallback()
            } else {
                (a, n)
            }
        }
        None => read_vpn_fallback(),
    };

    if let Ok(mut s) = state.lock() {
        s.vpn_active = active;
        s.vpn_name = name;
    }
    dirty.store(true, Ordering::Relaxed);
}

pub fn start_vpn_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let conn = match Connection::system() {
            Ok(c) => c,
            Err(e) => {
                log::warn!("[vpn] failed to connect to system bus: {e}, using fallback");
                // Fallback polling loop
                loop {
                    update_state(None, &state, &dirty);
                    std::thread::sleep(std::time::Duration::from_secs(10));
                }
            }
        };

        // Initial read
        update_state(Some(&conn), &state, &dirty);

        // Subscribe to NM signals for connection changes
        let rule = match MatchRule::builder()
            .msg_type(MessageType::Signal)
            .sender("org.freedesktop.NetworkManager")
        {
            Ok(b) => b.build(),
            Err(e) => {
                log::warn!("[vpn] failed to build match rule: {e}");
                return;
            }
        };

        let mut iter =
            match zbus::blocking::MessageIterator::for_match_rule(rule, &conn, Some(64)) {
                Ok(it) => it,
                Err(e) => {
                    log::warn!("[vpn] failed to create message iterator: {e}");
                    return;
                }
            };

        while let Some(Ok(_msg)) = iter.next() {
            update_state(Some(&conn), &state, &dirty);
        }
    });
}
