use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use crate::state::FullState;
use crate::poll::sh;

pub fn start_keyboard_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        loop {
            let output = sh("setxkbmap -query 2>/dev/null").unwrap_or_default();
            let mut layout = String::new();
            let mut variant = String::new();
            for line in output.lines() {
                if let Some(val) = line.strip_prefix("layout:") {
                    layout = val.trim().to_string();
                } else if let Some(val) = line.strip_prefix("variant:") {
                    variant = val.trim().to_string();
                }
            }
            if let Ok(mut s) = state.lock() {
                s.kb_layout = layout;
                s.kb_variant = variant;
            }
            dirty.store(true, Ordering::Relaxed);
            std::thread::sleep(std::time::Duration::from_secs(3));
        }
    });
}
