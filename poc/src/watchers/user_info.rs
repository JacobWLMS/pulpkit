use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use crate::state::FullState;

pub fn start_user_info_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let conn = match zbus::blocking::Connection::system() {
            Ok(c) => c,
            Err(_) => return,
        };

        let username = std::env::var("USER").unwrap_or_default();
        if username.is_empty() { return; }

        // Find user object path
        let reply = match conn.call_method(
            Some("org.freedesktop.Accounts"),
            "/org/freedesktop/Accounts",
            Some("org.freedesktop.Accounts"),
            "FindUserByName",
            &(username.as_str(),),
        ) {
            Ok(r) => r,
            Err(_) => return,
        };

        let user_path: zbus::zvariant::OwnedObjectPath = match reply.body() {
            Ok(p) => p,
            Err(_) => return,
        };

        // Read IconFile property
        let reply = match conn.call_method(
            Some("org.freedesktop.Accounts"),
            user_path.as_str(),
            Some("org.freedesktop.DBus.Properties"),
            "Get",
            &("org.freedesktop.Accounts.User", "IconFile"),
        ) {
            Ok(r) => r,
            Err(_) => return,
        };

        if let Ok(val) = reply.body::<zbus::zvariant::OwnedValue>() {
            if let Ok(icon_path) = String::try_from(val) {
                if let Ok(mut s) = state.lock() {
                    s.user_icon = icon_path;
                }
                dirty.store(true, Ordering::Relaxed);
            }
        }
    });
}
