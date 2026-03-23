use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::poll::sh;
use crate::state::FullState;

pub fn start_packagekit_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || loop {
        let count = check_updates();
        if let Ok(mut s) = state.lock() {
            s.updates_available = count;
        }
        dirty.store(true, Ordering::Relaxed);
        // Check every 30 minutes
        std::thread::sleep(std::time::Duration::from_secs(1800));
    });
}

fn check_updates() -> u32 {
    // Try checkupdates (Arch/CachyOS)
    if let Some(output) = sh("checkupdates 2>/dev/null") {
        return output.lines().count() as u32;
    }
    // Try apt (Debian/Ubuntu)
    if let Some(output) = sh("apt list --upgradable 2>/dev/null") {
        return output.lines().filter(|l| l.contains("upgradable")).count() as u32;
    }
    // Try dnf (Fedora)
    if let Some(output) = sh("dnf check-update --quiet 2>/dev/null") {
        return output.lines().filter(|l| !l.is_empty()).count() as u32;
    }
    0
}
