use std::io::BufRead;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::state::FullState;

fn read_ac_status() -> bool {
    // Check common AC adapter paths
    for name in &["AC", "AC0", "ACAD", "ADP1"] {
        let path = format!("/sys/class/power_supply/{name}/online");
        if let Ok(content) = std::fs::read_to_string(&path) {
            return content.trim() == "1";
        }
    }
    true // default to plugged in if no AC adapter found
}

pub fn start_ac_power_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        // Initial read
        if let Ok(mut s) = state.lock() {
            s.ac_plugged = read_ac_status();
        }
        dirty.store(true, Ordering::Relaxed);

        let Ok(mut child) = Command::new("udevadm")
            .args(["monitor", "--kernel", "--subsystem-match=power_supply"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        else {
            eprintln!("[pulpkit] udevadm power_supply monitor failed to start");
            return;
        };

        let stdout = child.stdout.take().unwrap();
        let reader = std::io::BufReader::new(stdout);
        for line in reader.lines() {
            let Ok(line) = line else { break };
            if line.contains("change") || line.contains("add") {
                std::thread::sleep(std::time::Duration::from_millis(100));
                if let Ok(mut s) = state.lock() {
                    s.ac_plugged = read_ac_status();
                }
                dirty.store(true, Ordering::Relaxed);
            }
        }
    });
}
