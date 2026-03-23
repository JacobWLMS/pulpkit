use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::state::FullState;

pub fn start_night_light_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || loop {
        let active =
            is_running("wlsunset") || is_running("gammastep") || is_running("redshift");
        if let Ok(mut s) = state.lock() {
            s.night_light_active = active;
        }
        dirty.store(true, Ordering::Relaxed);
        std::thread::sleep(std::time::Duration::from_secs(5));
    });
}

fn is_running(name: &str) -> bool {
    std::process::Command::new("pgrep")
        .arg("-x")
        .arg(name)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
