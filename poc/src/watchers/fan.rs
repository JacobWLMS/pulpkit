use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::state::FullState;

pub fn start_fan_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let fan_path = match find_fan_input() {
            Some(p) => p,
            None => {
                log::warn!("[fan] no fan sensor found in /sys/class/hwmon");
                return;
            }
        };

        loop {
            let rpm = read_fan_rpm(&fan_path);
            if let Ok(mut s) = state.lock() {
                s.fan_rpm = rpm;
            }
            dirty.store(true, Ordering::Relaxed);
            std::thread::sleep(std::time::Duration::from_secs(5));
        }
    });
}

fn find_fan_input() -> Option<String> {
    let Ok(entries) = std::fs::read_dir("/sys/class/hwmon") else {
        return None;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let fan_path = format!("{}/fan1_input", path.display());
        if std::path::Path::new(&fan_path).exists() {
            return Some(fan_path);
        }
    }
    None
}

fn read_fan_rpm(path: &str) -> u32 {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}
