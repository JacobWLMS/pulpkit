use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use zbus::blocking::Connection;
use zbus::zvariant::{OwnedValue, Value};
use zbus::{MatchRule, MessageType};

use crate::state::FullState;

const FCITX5_DEST: &str = "org.fcitx.Fcitx5.Controller1";
const FCITX5_PATH: &str = "/controller";
const FCITX5_IFACE: &str = "org.fcitx.Fcitx5.Controller1";

const IBUS_DEST: &str = "org.freedesktop.IBus";
const IBUS_PATH: &str = "/org/freedesktop/IBus";
const IBUS_IFACE: &str = "org.freedesktop.IBus";

pub fn start_input_method_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let conn = match Connection::session() {
            Ok(c) => c,
            Err(_) => {
                // No session bus — no IM detection possible
                return;
            }
        };

        // Try fcitx5 first
        if try_fcitx5(&conn, &state, &dirty) {
            return;
        }

        // Try ibus
        if try_ibus(&conn, &state, &dirty) {
            return;
        }

        // Neither available
        if let Ok(mut s) = state.lock() {
            s.im_active = false;
            s.im_name = String::new();
        }
        dirty.store(true, Ordering::Relaxed);
    });
}

fn read_string_property(
    conn: &Connection,
    dest: &str,
    path: &str,
    iface: &str,
    prop: &str,
) -> Option<String> {
    let reply = conn
        .call_method(
            Some(dest),
            path,
            Some("org.freedesktop.DBus.Properties"),
            "Get",
            &(iface, prop),
        )
        .ok()?;
    let val: OwnedValue = reply.body().ok()?;
    let inner: Value = (&val).into();
    match inner {
        Value::Str(s) => Some(s.to_string()),
        _ => None,
    }
}

/// Try to connect to fcitx5 and watch for changes. Returns true if fcitx5 is available.
fn try_fcitx5(
    conn: &Connection,
    state: &Arc<Mutex<FullState>>,
    dirty: &Arc<AtomicBool>,
) -> bool {
    // Check if fcitx5 is reachable by reading CurrentInputMethod
    let Some(im_name) = read_string_property(conn, FCITX5_DEST, FCITX5_PATH, FCITX5_IFACE, "CurrentInputMethod") else {
        return false;
    };

    if let Ok(mut s) = state.lock() {
        s.im_active = true;
        s.im_name = im_name;
    }
    dirty.store(true, Ordering::Relaxed);

    // Subscribe to PropertiesChanged on the fcitx5 controller
    let rule = match MatchRule::builder()
        .msg_type(MessageType::Signal)
        .sender(FCITX5_DEST)
        .ok()
        .and_then(|b| {
            b.interface("org.freedesktop.DBus.Properties")
                .ok()
        })
        .map(|b| b.build())
    {
        Some(r) => r,
        None => return true,
    };

    let mut iter = match zbus::blocking::MessageIterator::for_match_rule(rule, conn, Some(16)) {
        Ok(it) => it,
        Err(_) => return true,
    };

    while let Some(Ok(_msg)) = iter.next() {
        // Re-read the current input method on any property change
        if let Some(name) = read_string_property(conn, FCITX5_DEST, FCITX5_PATH, FCITX5_IFACE, "CurrentInputMethod") {
            if let Ok(mut s) = state.lock() {
                s.im_active = true;
                s.im_name = name;
            }
            dirty.store(true, Ordering::Relaxed);
        }
    }

    true
}

/// Try to connect to ibus and watch for changes. Returns true if ibus is available.
fn try_ibus(
    conn: &Connection,
    state: &Arc<Mutex<FullState>>,
    dirty: &Arc<AtomicBool>,
) -> bool {
    // Check if ibus is reachable
    let Some(engine) = read_string_property(conn, IBUS_DEST, IBUS_PATH, IBUS_IFACE, "GlobalEngine") else {
        return false;
    };

    if let Ok(mut s) = state.lock() {
        s.im_active = true;
        s.im_name = engine;
    }
    dirty.store(true, Ordering::Relaxed);

    // Subscribe to PropertiesChanged on ibus
    let rule = match MatchRule::builder()
        .msg_type(MessageType::Signal)
        .sender(IBUS_DEST)
        .ok()
        .and_then(|b| {
            b.interface("org.freedesktop.DBus.Properties")
                .ok()
        })
        .map(|b| b.build())
    {
        Some(r) => r,
        None => return true,
    };

    let mut iter = match zbus::blocking::MessageIterator::for_match_rule(rule, conn, Some(16)) {
        Ok(it) => it,
        Err(_) => return true,
    };

    while let Some(Ok(_msg)) = iter.next() {
        if let Some(name) = read_string_property(conn, IBUS_DEST, IBUS_PATH, IBUS_IFACE, "GlobalEngine") {
            if let Ok(mut s) = state.lock() {
                s.im_active = true;
                s.im_name = name;
            }
            dirty.store(true, Ordering::Relaxed);
        }
    }

    true
}
