use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use crate::state::FullState;

pub fn start_focus_tracker(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let mut last_app = String::new();
        let mut secs: u32 = 0;
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
            if let Ok(mut s) = state.lock() {
                if s.active_app_id == last_app {
                    secs += 1;
                } else {
                    last_app = s.active_app_id.clone();
                    secs = 0;
                }
                s.focused_app_time_secs = secs;
            }
            dirty.store(true, Ordering::Relaxed);
        }
    });
}
