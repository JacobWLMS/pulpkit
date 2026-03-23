use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use crate::state::FullState;

struct ShellService {
    state: Arc<Mutex<FullState>>,
    dirty: Arc<AtomicBool>,
}

#[zbus::dbus_interface(name = "org.pulpkit.Shell")]
impl ShellService {
    fn get_state(&self) -> String {
        let s = self.state.lock().unwrap();
        serde_json::to_string(&*s).unwrap_or_else(|_| "{}".into())
    }

    fn set_custom(&self, key: String, value: String) {
        let parsed: serde_json::Value = serde_json::from_str(&value).unwrap_or(serde_json::Value::String(value));
        if let Ok(mut s) = self.state.lock() {
            s.custom.insert(key, parsed);
        }
        self.dirty.store(true, Ordering::Relaxed);
    }

    fn toggle_popup(&self, name: String) {
        if let Ok(mut s) = self.state.lock() {
            if s.popup == name {
                s.popup.clear();
            } else {
                s.popup = name;
            }
        }
        self.dirty.store(true, Ordering::Relaxed);
    }

    fn dismiss(&self) {
        if let Ok(mut s) = self.state.lock() {
            s.popup.clear();
        }
        self.dirty.store(true, Ordering::Relaxed);
    }

    fn exec(&self, cmd: String) {
        let _ = std::process::Command::new("sh")
            .args(["-c", &cmd])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
    }
}

pub fn start_dbus_service(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        rt.block_on(async {
            let service = ShellService { state, dirty };
            let _conn = match zbus::ConnectionBuilder::session()
                .expect("session bus")
                .name("org.pulpkit.Shell")
                .expect("name")
                .serve_at("/org/pulpkit/Shell", service)
                .expect("serve")
                .build()
                .await
            {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("[pulpkit] shell DBus service failed: {e}");
                    return;
                }
            };
            // Keep alive
            loop { tokio::time::sleep(std::time::Duration::from_secs(3600)).await; }
        });
    });
}
