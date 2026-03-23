use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use crate::state::FullState;

pub fn start_caffeine_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into());
        let marker = format!("{runtime_dir}/pulpkit-caffeine");
        loop {
            let active = std::path::Path::new(&marker).exists();
            if let Ok(mut s) = state.lock() {
                s.caffeine_active = active;
            }
            dirty.store(true, Ordering::Relaxed);
            std::thread::sleep(std::time::Duration::from_secs(5));
        }
    });
}
