use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::poll::sh;
use crate::state::{FullState, ProcessInfo};

pub fn start_top_procs_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || loop {
        let procs = read_top_procs();
        if let Ok(mut s) = state.lock() {
            s.top_procs = procs;
        }
        dirty.store(true, Ordering::Relaxed);
        std::thread::sleep(std::time::Duration::from_secs(5));
    });
}

fn read_top_procs() -> Vec<ProcessInfo> {
    let output = sh("ps aux --sort=-%cpu 2>/dev/null").unwrap_or_default();
    let mut procs = Vec::new();
    for line in output.lines().skip(1).take(5) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 11 {
            let pid = parts[1].parse::<u32>().unwrap_or(0);
            let cpu_pct = parts[2].parse::<f32>().unwrap_or(0.0);
            let mem_kb = parts[5].parse::<u32>().unwrap_or(0); // RSS in KB
            let name = parts[10..].join(" ");
            // Skip kernel threads and very low CPU
            if !name.starts_with('[') {
                procs.push(ProcessInfo {
                    name,
                    pid,
                    cpu_pct,
                    mem_mb: mem_kb / 1024,
                });
            }
        }
    }
    procs.truncate(5);
    procs
}
