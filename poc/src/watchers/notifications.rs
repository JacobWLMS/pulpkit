use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::state::{FullState, Notification};

struct NotificationServer {
    state: Arc<Mutex<FullState>>,
    dirty: Arc<AtomicBool>,
    next_id: AtomicU32,
}

#[zbus::dbus_interface(name = "org.freedesktop.Notifications")]
impl NotificationServer {
    fn get_capabilities(&self) -> Vec<String> {
        vec!["body".into(), "actions".into(), "icon-static".into()]
    }

    fn notify(
        &self,
        app_name: String,
        replaces_id: u32,
        app_icon: String,
        summary: String,
        body: String,
        _actions: Vec<String>,
        _hints: std::collections::HashMap<String, zbus::zvariant::OwnedValue>,
        expire_timeout: i32,
    ) -> u32 {
        let id = if replaces_id > 0 {
            replaces_id
        } else {
            self.next_id.fetch_add(1, Ordering::Relaxed)
        };

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let notif = Notification {
            id,
            app_name,
            summary,
            body,
            icon: app_icon,
            timestamp: now,
        };

        if let Ok(mut s) = self.state.lock() {
            s.notifications.retain(|n| n.id != id);
            s.notifications.push(notif);
            s.notif_count = s.notifications.len() as u32;
        }
        self.dirty.store(true, Ordering::Relaxed);

        // Auto-expire after timeout (if positive)
        if expire_timeout > 0 {
            let state = self.state.clone();
            let dirty = self.dirty.clone();
            let timeout = expire_timeout as u64;
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(timeout));
                if let Ok(mut s) = state.lock() {
                    s.notifications.retain(|n| n.id != id);
                    s.notif_count = s.notifications.len() as u32;
                }
                dirty.store(true, Ordering::Relaxed);
            });
        }

        id
    }

    fn close_notification(&self, id: u32) {
        if let Ok(mut s) = self.state.lock() {
            s.notifications.retain(|n| n.id != id);
            s.notif_count = s.notifications.len() as u32;
        }
        self.dirty.store(true, Ordering::Relaxed);
    }

    fn get_server_information(&self) -> (String, String, String, String) {
        (
            "pulpkit".into(),
            "pulpkit".into(),
            "0.1".into(),
            "1.2".into(),
        )
    }
}

pub fn start_notification_daemon(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        rt.block_on(async {
            let server = NotificationServer {
                state,
                dirty,
                next_id: AtomicU32::new(1),
            };

            let conn = match zbus::ConnectionBuilder::session()
                .expect("session bus")
                .name("org.freedesktop.Notifications")
                .expect("name")
                .serve_at("/org/freedesktop/Notifications", server)
                .expect("serve")
                .build()
                .await
            {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("[pulpkit] notifications daemon failed: {e}");
                    return;
                }
            };

            // Keep the connection alive — zbus processes messages in the background
            // while the connection exists.
            let _conn = conn;
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            }
        });
    });
}
