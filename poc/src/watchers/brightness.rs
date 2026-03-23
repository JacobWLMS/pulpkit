use std::io::BufRead;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use crate::state::FullState;
use crate::poll::poll_bri;

pub fn start_brightness_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        // Initial read
        if let Ok(mut s) = state.lock() { s.bright = poll_bri(); }
        dirty.store(true, Ordering::Relaxed);

        // udevadm monitor for backlight changes
        let Ok(mut child) = Command::new("udevadm")
            .args(["monitor", "--kernel", "--subsystem-match=backlight"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        else {
            eprintln!("[pulpkit] udevadm monitor failed to start");
            return;
        };
        let stdout = child.stdout.take().unwrap();
        let reader = std::io::BufReader::new(stdout);
        for line in reader.lines() {
            let Ok(line) = line else { break };
            if line.contains("change") {
                // Small delay for the value to settle
                std::thread::sleep(std::time::Duration::from_millis(50));
                if let Ok(mut s) = state.lock() { s.bright = poll_bri(); }
                dirty.store(true, Ordering::Relaxed);
            }
        }
    });
}
