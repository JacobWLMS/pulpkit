use std::io::BufRead;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::state::FullState;

pub fn start_clipboard_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let Ok(mut child) = Command::new("wl-paste")
            .args(["--watch", "--type", "text/plain", "cat"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        else {
            eprintln!("[pulpkit] wl-paste --watch failed to start");
            return;
        };

        let stdout = child.stdout.take().unwrap();
        let reader = std::io::BufReader::new(stdout);
        for line in reader.lines() {
            let Ok(line) = line else { break };
            let truncated = if line.len() > 200 {
                format!("{}…", &line[..200])
            } else {
                line
            };
            if let Ok(mut s) = state.lock() {
                s.clipboard_text = truncated;
            }
            dirty.store(true, Ordering::Relaxed);
        }
    });
}
