use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::state::FullState;

pub fn start_load_avg_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || loop {
        let (l1, l5, l15) = read_load_avg();
        if let Ok(mut s) = state.lock() {
            s.load_1 = l1;
            s.load_5 = l5;
            s.load_15 = l15;
        }
        dirty.store(true, Ordering::Relaxed);
        std::thread::sleep(std::time::Duration::from_secs(5));
    });
}

fn read_load_avg() -> (f32, f32, f32) {
    let Ok(content) = std::fs::read_to_string("/proc/loadavg") else {
        return (0.0, 0.0, 0.0);
    };
    let parts: Vec<&str> = content.split_whitespace().collect();
    if parts.len() < 3 {
        return (0.0, 0.0, 0.0);
    }
    let l1 = parts[0].parse::<f32>().unwrap_or(0.0);
    let l5 = parts[1].parse::<f32>().unwrap_or(0.0);
    let l15 = parts[2].parse::<f32>().unwrap_or(0.0);
    (l1, l5, l15)
}
