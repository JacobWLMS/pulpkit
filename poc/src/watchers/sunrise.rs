use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::poll::sh;
use crate::state::FullState;

pub fn start_sunrise_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || loop {
        if let Some((rise, set)) = fetch_sunrise_sunset() {
            if let Ok(mut s) = state.lock() {
                s.sunrise = rise;
                s.sunset = set;
            }
            dirty.store(true, Ordering::Relaxed);
        }
        std::thread::sleep(std::time::Duration::from_secs(3600));
    });
}

fn fetch_sunrise_sunset() -> Option<(String, String)> {
    let output = sh("curl -s 'wttr.in/?format=%S|%s' 2>/dev/null")?;
    let parts: Vec<&str> = output.splitn(2, '|').collect();
    if parts.len() < 2 {
        return None;
    }
    let rise = parts[0].trim().to_string();
    let set = parts[1].trim().to_string();
    if rise.is_empty() || set.is_empty() {
        return None;
    }
    Some((rise, set))
}
