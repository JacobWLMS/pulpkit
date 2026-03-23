use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use zbus::blocking::{Connection, MessageIterator};
use zbus::zvariant::{OwnedValue, Value};
use zbus::{MatchRule, MessageType};

use crate::state::FullState;

const LOGIND_DEST: &str = "org.freedesktop.login1";
const LOGIND_MANAGER_PATH: &str = "/org/freedesktop/login1";
const SESSION_IFACE: &str = "org.freedesktop.login1.Session";
const MANAGER_IFACE: &str = "org.freedesktop.login1.Manager";

/// Read a boolean property from a logind session object.
fn read_session_bool(conn: &Connection, session_path: &str, prop: &str) -> bool {
    conn.call_method(
        Some(LOGIND_DEST),
        session_path,
        Some("org.freedesktop.DBus.Properties"),
        "Get",
        &(SESSION_IFACE, prop),
    )
    .ok()
    .and_then(|reply| reply.body::<OwnedValue>().ok())
    .and_then(|ov| {
        let val: Value = (&ov).into();
        match val {
            Value::Bool(b) => Some(b),
            _ => None,
        }
    })
    .unwrap_or(false)
}

/// Re-read session properties and update state.
fn read_session_state(
    conn: &Connection,
    session_path: &str,
    state: &Arc<Mutex<FullState>>,
    dirty: &Arc<AtomicBool>,
) {
    let locked = read_session_bool(conn, session_path, "LockedHint");
    let idle = read_session_bool(conn, session_path, "IdleHint");

    if let Ok(mut s) = state.lock() {
        s.session_locked = locked;
        s.session_idle = idle;
    }
    dirty.store(true, Ordering::Relaxed);
}

pub fn start_logind_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let conn = match Connection::system() {
            Ok(c) => c,
            Err(e) => {
                log::warn!("[logind] failed to connect to system bus: {e}");
                return;
            }
        };

        // Resolve the current session path via GetSession("auto")
        let session_path: String = match conn.call_method(
            Some(LOGIND_DEST),
            LOGIND_MANAGER_PATH,
            Some(MANAGER_IFACE),
            "GetSession",
            &"auto",
        ) {
            Ok(reply) => match reply.body::<zbus::zvariant::OwnedObjectPath>() {
                Ok(p) => p.to_string(),
                Err(e) => {
                    log::warn!("[logind] failed to parse session path: {e}");
                    return;
                }
            },
            Err(e) => {
                log::warn!("[logind] GetSession(auto) failed: {e}");
                return;
            }
        };

        log::info!("[logind] watching session: {session_path}");

        // Initial read of session properties
        read_session_state(&conn, &session_path, &state, &dirty);

        // Subscribe to all signals from org.freedesktop.login1.
        // This captures both PropertiesChanged on the session and
        // PrepareForSleep on the manager object.
        let rule = match MatchRule::builder()
            .msg_type(MessageType::Signal)
            .sender(LOGIND_DEST)
        {
            Ok(b) => b.build(),
            Err(e) => {
                log::warn!("[logind] failed to build match rule: {e}");
                return;
            }
        };

        let mut iter = match MessageIterator::for_match_rule(rule, &conn, Some(16)) {
            Ok(it) => it,
            Err(e) => {
                log::warn!("[logind] failed to create message iterator: {e}");
                return;
            }
        };

        while let Some(Ok(msg)) = iter.next() {
            let header = match msg.header() {
                Ok(h) => h,
                Err(_) => continue,
            };

            let iface = header
                .interface()
                .ok()
                .flatten()
                .map(|i| i.as_str().to_string())
                .unwrap_or_default();
            let member = header
                .member()
                .ok()
                .flatten()
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();

            match (iface.as_str(), member.as_str()) {
                // PropertiesChanged on our session — re-read LockedHint and IdleHint
                ("org.freedesktop.DBus.Properties", "PropertiesChanged") => {
                    let path = header
                        .path()
                        .ok()
                        .flatten()
                        .map(|p| p.as_str().to_string())
                        .unwrap_or_default();
                    if path == session_path {
                        read_session_state(&conn, &session_path, &state, &dirty);
                    }
                }
                // PrepareForSleep(bool) on the manager
                (MANAGER_IFACE, "PrepareForSleep") => {
                    let sleeping = msg.body::<bool>().unwrap_or(false);
                    if let Ok(mut s) = state.lock() {
                        s.preparing_sleep = sleeping;
                    }
                    dirty.store(true, Ordering::Relaxed);

                    // On resume, re-read session state as it may have changed
                    if !sleeping {
                        read_session_state(&conn, &session_path, &state, &dirty);
                    }
                }
                _ => {}
            }
        }
    });
}
