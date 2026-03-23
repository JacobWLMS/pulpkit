use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::poll::sh;
use crate::state::FullState;

pub fn start_screen_share_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || loop {
        let sharing = is_screen_sharing();
        if let Ok(mut s) = state.lock() {
            s.screen_sharing = sharing;
        }
        dirty.store(true, Ordering::Relaxed);
        std::thread::sleep(std::time::Duration::from_secs(3));
    });
}

fn is_screen_sharing() -> bool {
    // Check for active screen capture nodes in PipeWire
    let output = sh("pw-cli ls Node 2>/dev/null").unwrap_or_default();
    output.contains("Screen") || output.contains("screencast") || output.contains("xdg-desktop-portal")
}
