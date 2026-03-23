use std::collections::HashMap;
use std::io::BufRead;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::poll::{poll_windows, poll_ws};
use crate::state::FullState;

pub fn start_niri_stream(
    state: Arc<Mutex<FullState>>,
    dirty: Arc<AtomicBool>,
    icon_cache: HashMap<String, String>,
) {
    std::thread::spawn(move || {
        let Ok(mut child) = Command::new("niri")
            .args(["msg", "event-stream"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        else {
            eprintln!("[pulpkit] niri event-stream failed to start");
            return;
        };

        let stdout = child.stdout.take().unwrap();
        let reader = std::io::BufReader::new(stdout);
        for line in reader.lines() {
            let Ok(line) = line else { break };
            if line.contains("Workspace") || line.contains("Window") {
                let ws = poll_ws();
                let wins = poll_windows(&icon_cache);
                let active_title = wins
                    .iter()
                    .find(|w| w.focused)
                    .map(|w| w.title.clone())
                    .unwrap_or_default();
                let active_app_id = wins
                    .iter()
                    .find(|w| w.focused)
                    .map(|w| w.app_id.clone())
                    .unwrap_or_default();
                if let Ok(mut s) = state.lock() {
                    s.ws = ws;
                    s.windows = wins;
                    s.active_title = active_title;
                    s.active_app_id = active_app_id;
                }
                dirty.store(true, Ordering::Relaxed);
            }
        }
    });
}
