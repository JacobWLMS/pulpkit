use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::poll::sh;
use crate::state::FullState;

pub fn start_ssh_sessions_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || loop {
        let count = count_ssh_sessions();
        if let Ok(mut s) = state.lock() {
            s.ssh_sessions = count;
        }
        dirty.store(true, Ordering::Relaxed);
        std::thread::sleep(std::time::Duration::from_secs(10));
    });
}

fn count_ssh_sessions() -> u32 {
    sh("ss -tn sport = :22 2>/dev/null | tail -n +2 | wc -l")
        .and_then(|s| s.trim().parse::<u32>().ok())
        .unwrap_or(0)
}
