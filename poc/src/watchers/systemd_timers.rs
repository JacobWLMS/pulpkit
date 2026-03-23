use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::poll::sh;
use crate::state::{FullState, TimerInfo};

pub fn start_systemd_timers_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || loop {
        let timers = read_timers();
        if let Ok(mut s) = state.lock() {
            s.timers = timers;
        }
        dirty.store(true, Ordering::Relaxed);
        std::thread::sleep(std::time::Duration::from_secs(60));
    });
}

fn read_timers() -> Vec<TimerInfo> {
    let output =
        sh("systemctl list-timers --no-pager --plain 2>/dev/null").unwrap_or_default();
    let mut timers = Vec::new();
    // Skip the header line; columns: NEXT LEFT LAST PASSED UNIT ACTIVATES
    for line in output.lines().skip(1).take(10) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        // Typical plain output has >=6 columns
        // NEXT is first 3-4 words (day date time tz), then LEFT, LAST (3-4 words), PASSED, UNIT, ACTIVATES
        // With --plain, UNIT is second-to-last and ACTIVATES is last
        if parts.len() < 2 {
            continue;
        }
        // The last two columns are UNIT and ACTIVATES
        let unit = parts[parts.len() - 2].to_string();
        let activates = parts[parts.len() - 1].to_string();
        // If unit ends with .timer, use it; otherwise the line may be a footer
        if !unit.contains('.') {
            continue;
        }
        // NEXT is roughly the first few columns, LAST is in the middle
        // For simplicity, grab NEXT as first 4 words joined, LAST as "n/a" or best-effort
        let next_trigger = if parts.len() >= 6 {
            parts[..4].join(" ")
        } else {
            "n/a".into()
        };
        let last_trigger = if parts.len() >= 10 {
            parts[4..8].join(" ")
        } else {
            "n/a".into()
        };
        timers.push(TimerInfo {
            name: activates,
            next_trigger,
            last_trigger,
        });
    }
    timers.truncate(10);
    timers
}
