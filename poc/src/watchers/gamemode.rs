use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use zbus::blocking::Connection;
use zbus::zvariant::OwnedValue;
use zbus::MatchRule;
use zbus::MessageType;

use crate::state::FullState;

const GM_DEST: &str = "com.feralinteractive.GameMode";
const GM_PATH: &str = "/com/feralinteractive/GameMode";
const GM_IFACE: &str = "com.feralinteractive.GameMode";

/// Try to read ClientCount via D-Bus Properties. Returns None if the property
/// is unavailable (older GameMode versions).
fn read_client_count(conn: &Connection) -> Option<i32> {
    let reply = conn
        .call_method(
            Some(GM_DEST),
            GM_PATH,
            Some("org.freedesktop.DBus.Properties"),
            "Get",
            &(GM_IFACE, "ClientCount"),
        )
        .ok()?;

    let ov: OwnedValue = reply.body().ok()?;
    // ClientCount is an i32 inside a variant
    i32::try_from(ov).ok()
}

/// Update gamemode_active in shared state.
fn update_state(conn: &Connection, state: &Arc<Mutex<FullState>>, dirty: &Arc<AtomicBool>) {
    let active = read_client_count(conn).map(|c| c > 0).unwrap_or(false);
    if let Ok(mut s) = state.lock() {
        s.gamemode_active = active;
    }
    dirty.store(true, Ordering::Relaxed);
}

pub fn start_gamemode_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let conn = match Connection::session() {
            Ok(c) => c,
            Err(e) => {
                log::warn!("[gamemode] failed to connect to session bus: {e}");
                return;
            }
        };

        // Check if GameMode service is available at all
        if read_client_count(&conn).is_none() {
            log::info!("[gamemode] GameMode not available, watcher disabled");
            return;
        }

        // Initial read
        update_state(&conn, &state, &dirty);

        // Listen for GameRegistered / GameUnregistered signals from the GameMode interface
        let rule = match MatchRule::builder()
            .msg_type(MessageType::Signal)
            .interface(GM_IFACE)
        {
            Ok(b) => b.build(),
            Err(e) => {
                log::warn!("[gamemode] failed to build match rule: {e}");
                return;
            }
        };

        let mut iter =
            match zbus::blocking::MessageIterator::for_match_rule(rule, &conn, Some(16)) {
                Ok(it) => it,
                Err(e) => {
                    log::warn!("[gamemode] failed to create message iterator: {e}");
                    return;
                }
            };

        // On any signal from GameMode, re-read client count
        while let Some(Ok(_msg)) = iter.next() {
            update_state(&conn, &state, &dirty);
        }
    });
}
