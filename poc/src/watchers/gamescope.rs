use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::state::FullState;

pub fn start_gamescope_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        // Env var is set once at session start and won't change, so read it once.
        let env_active = std::env::var("GAMESCOPE_WAYLAND_DISPLAY").is_ok();

        loop {
            let active = env_active || is_gamescope_running();
            if let Ok(mut s) = state.lock() {
                s.gamescope_active = active;
            }
            dirty.store(true, Ordering::Relaxed);
            std::thread::sleep(std::time::Duration::from_secs(5));
        }
    });
}

fn is_gamescope_running() -> bool {
    std::process::Command::new("pgrep")
        .arg("-x")
        .arg("gamescope")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
