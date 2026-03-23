use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::state::FullState;

pub fn start_swap_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || loop {
        let (used, total) = read_swap();
        if let Ok(mut s) = state.lock() {
            s.swap_used_mb = used;
            s.swap_total_mb = total;
        }
        dirty.store(true, Ordering::Relaxed);
        std::thread::sleep(std::time::Duration::from_secs(5));
    });
}

fn read_swap() -> (u32, u32) {
    let Ok(content) = std::fs::read_to_string("/proc/meminfo") else {
        return (0, 0);
    };
    let mut total_kb: u64 = 0;
    let mut free_kb: u64 = 0;
    for line in content.lines() {
        if let Some(val) = line.strip_prefix("SwapTotal:") {
            total_kb = val
                .trim()
                .split_whitespace()
                .next()
                .and_then(|n| n.parse().ok())
                .unwrap_or(0);
        } else if let Some(val) = line.strip_prefix("SwapFree:") {
            free_kb = val
                .trim()
                .split_whitespace()
                .next()
                .and_then(|n| n.parse().ok())
                .unwrap_or(0);
        }
        if total_kb > 0 && free_kb > 0 {
            break;
        }
    }
    let total_mb = (total_kb / 1024) as u32;
    let used_mb = ((total_kb.saturating_sub(free_kb)) / 1024) as u32;
    (used_mb, total_mb)
}
