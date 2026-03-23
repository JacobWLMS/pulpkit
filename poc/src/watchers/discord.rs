use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::state::FullState;

pub fn start_discord_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into());
        loop {
            let mut activity = String::new();
            // Check discord-ipc-0 through discord-ipc-9
            for i in 0..10 {
                let path = format!("{runtime_dir}/discord-ipc-{i}");
                if std::path::Path::new(&path).exists() {
                    activity = "Discord running".to_string();
                    break;
                }
            }
            if let Ok(mut s) = state.lock() {
                s.discord_activity = activity;
            }
            dirty.store(true, Ordering::Relaxed);
            std::thread::sleep(std::time::Duration::from_secs(5));
        }
    });
}
