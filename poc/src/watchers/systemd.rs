use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use zbus::blocking::{Connection, MessageIterator};
use zbus::{MatchRule, MessageType};

use crate::state::FullState;

const SYSTEMD_DEST: &str = "org.freedesktop.systemd1";
const SYSTEMD_PATH: &str = "/org/freedesktop/systemd1";
const MANAGER_IFACE: &str = "org.freedesktop.systemd1.Manager";

/// Call ListUnits() and return names of units whose active_state == "failed".
fn list_failed_units(conn: &Connection) -> Vec<String> {
    // ListUnits returns a(ssssssouso):
    //   name, description, load_state, active_state, sub_state,
    //   followed, unit_path, job_id, job_type, job_path
    let reply = match conn.call_method(
        Some(SYSTEMD_DEST),
        SYSTEMD_PATH,
        Some(MANAGER_IFACE),
        "ListUnits",
        &(),
    ) {
        Ok(r) => r,
        Err(e) => {
            log::warn!("[systemd] ListUnits failed: {e}");
            return vec![];
        }
    };

    let units: Vec<(
        String,               // name
        String,               // description
        String,               // load_state
        String,               // active_state
        String,               // sub_state
        String,               // followed
        zbus::zvariant::OwnedObjectPath, // unit_path
        u32,                  // job_id
        String,               // job_type
        zbus::zvariant::OwnedObjectPath, // job_path
    )> = match reply.body() {
        Ok(v) => v,
        Err(e) => {
            log::warn!("[systemd] failed to parse ListUnits body: {e}");
            return vec![];
        }
    };

    units
        .into_iter()
        .filter(|u| u.3 == "failed")
        .map(|u| u.0)
        .collect()
}

fn update_state(
    failed: Vec<String>,
    state: &Arc<Mutex<FullState>>,
    dirty: &Arc<AtomicBool>,
) {
    if let Ok(mut s) = state.lock() {
        s.failed_unit_count = failed.len() as u32;
        s.failed_units = failed;
    }
    dirty.store(true, Ordering::Relaxed);
}

pub fn start_systemd_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let conn = match Connection::system() {
            Ok(c) => c,
            Err(e) => {
                log::warn!("[systemd] failed to connect to system bus: {e}");
                return;
            }
        };

        // Initial read
        let failed = list_failed_units(&conn);
        update_state(failed, &state, &dirty);

        // Subscribe to systemd1 signals: UnitNew, UnitRemoved, PropertiesChanged
        let rule = match MatchRule::builder()
            .msg_type(MessageType::Signal)
            .sender(SYSTEMD_DEST)
        {
            Ok(b) => b.build(),
            Err(e) => {
                log::warn!("[systemd] failed to build match rule: {e}");
                return;
            }
        };

        let mut iter = match MessageIterator::for_match_rule(rule, &conn, Some(16)) {
            Ok(it) => it,
            Err(e) => {
                log::warn!("[systemd] failed to create message iterator: {e}");
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
                (MANAGER_IFACE, "UnitNew")
                | (MANAGER_IFACE, "UnitRemoved")
                | ("org.freedesktop.DBus.Properties", "PropertiesChanged") => {
                    let failed = list_failed_units(&conn);
                    update_state(failed, &state, &dirty);
                }
                _ => {}
            }
        }

        log::warn!("[systemd] signal iterator ended unexpectedly");
    });
}
