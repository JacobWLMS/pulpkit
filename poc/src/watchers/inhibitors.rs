use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use crate::state::{InhibitorInfo, FullState};

pub fn start_inhibitor_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let conn = match zbus::blocking::Connection::system() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[pulpkit] inhibitors: failed to connect to system bus: {e}");
                return;
            }
        };

        loop {
            let inhibitors = read_inhibitors(&conn);
            if let Ok(mut s) = state.lock() {
                s.inhibitors = inhibitors;
            }
            dirty.store(true, Ordering::Relaxed);
            std::thread::sleep(std::time::Duration::from_secs(10));
        }
    });
}

fn read_inhibitors(conn: &zbus::blocking::Connection) -> Vec<InhibitorInfo> {
    let reply = match conn.call_method(
        Some("org.freedesktop.login1"),
        "/org/freedesktop/login1",
        Some("org.freedesktop.login1.Manager"),
        "ListInhibitors",
        &(),
    ) {
        Ok(r) => r,
        Err(_) => return vec![],
    };

    // Body is a(ssssuu) - array of (what, who, why, mode, uid, pid)
    let items: Vec<(String, String, String, String, u32, u32)> = match reply.body() {
        Ok(v) => v,
        Err(_) => return vec![],
    };

    items.into_iter().map(|(what, who, why, _mode, _uid, _pid)| {
        InhibitorInfo { what, who, why }
    }).collect()
}
